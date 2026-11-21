use bookworm::{AppState, cron_registry, observability, routes};
use cja::{
    color_eyre,
    eyes_manifest::ProcessIdentity,
    setup::TracingConfig,
    tasks::{ShutdownBudget, Supervisor},
};
use std::time::Duration;
use tracing::info;

/// Bookworm runs no job worker, so the whole budget is exit grace: time for
/// in-flight HTTP requests and the active cron tick (the weekly email, sent
/// directly from the cron worker) to finish. Fly's `kill_timeout` is 60s; 30s
/// here plus 5s for the Eyes heartbeat report and 3s for telemetry flush
/// totals 38s, leaving 22s under Fly's 60s kill timeout.
const SHUTDOWN_BUDGET: ShutdownBudget = ShutdownBudget {
    job_drain: Duration::ZERO,
    exit_grace: Duration::from_secs(30),
};

fn main() -> color_eyre::Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()?
        .block_on(run_application())
}

async fn run_application() -> cja::Result<()> {
    let identity = ProcessIdentity::new(observability::ROLE);
    let telemetry = TracingConfig::new(observability::APP_NAME)
        .process(identity.clone())
        .version(env!("CARGO_PKG_VERSION"))
        .init()?;
    let result = run_services(identity).await;
    if let Err(error) = &result {
        tracing::error!(error = %format!("{error:#}"), "Bookworm stopped with an error");
    }
    info!("Flushing telemetry");
    if let Err(error) = telemetry.shutdown().await {
        eprintln!("Failed to flush telemetry: {error:#}");
    }
    result
}

async fn run_services(identity: ProcessIdentity) -> cja::Result<()> {
    let app_state = AppState::from_env().await?;
    let server_enabled = is_feature_enabled("SERVER");
    let cron_registry = is_feature_enabled("CRON").then(cron_registry);
    if !server_enabled && cron_registry.is_none() {
        info!("No application tasks enabled");
        return Ok(());
    }
    let cron_timezone = if let Some(registry) = cron_registry.as_ref() {
        let timezone = observability::cron_timezone()?;
        let boot = cja::posthog_manifest::BootManifest::new(
            observability::APP_NAME,
            registry,
            &timezone,
            &[],
            &[],
        );
        let boot = if let Some(sha) = observability::git_sha() {
            boot.git_sha(sha)
        } else {
            boot
        };
        cja::posthog_manifest::emit_boot_manifest::<bookworm::Jobs, AppState>(&app_state, &boot)?;
        Some(timezone)
    } else {
        None
    };
    let manifest = observability::manifest(identity, cron_registry.as_ref())
        .map_err(|error| color_eyre::eyre::eyre!(error))?;
    // Registers SIGTERM/SIGINT now, so a signal during Eyes registration is not lost.
    let mut supervisor = Supervisor::new(SHUTDOWN_BUDGET)?;
    let heartbeat = observability::start(&manifest).await;
    spawn_application_tasks(
        &mut supervisor,
        &app_state,
        server_enabled,
        cron_registry,
        cron_timezone,
    );
    let result = supervisor.run().await;
    if let Some(heartbeat) = heartbeat
        && let Err(error) = heartbeat.shutdown().await
    {
        tracing::warn!(%error, "Eyes process shutdown report failed");
    }
    result
}

fn spawn_application_tasks(
    supervisor: &mut Supervisor,
    app_state: &AppState,
    server_enabled: bool,
    cron_registry: Option<cja::cron::CronRegistry<AppState>>,
    cron_timezone: Option<cja::jiff::tz::TimeZone>,
) {
    let shutdown = supervisor.shutdown_token();
    if server_enabled {
        info!("Server Enabled");
        supervisor.spawn(
            "HTTP server",
            cja::server::run_server_until(
                routes(app_state.clone()),
                shutdown.clone().cancelled_owned(),
            ),
        );
    } else {
        info!("Server Disabled");
    }
    if let (Some(cron_registry), Some(timezone)) = (cron_registry, cron_timezone) {
        info!("Cron Enabled");
        supervisor.spawn(
            "Cron worker",
            bookworm::run_cron(app_state.clone(), cron_registry, timezone, shutdown),
        );
    } else {
        info!("Cron Disabled");
    }
}

fn is_feature_enabled(feature: &str) -> bool {
    std::env::var(format!("{feature}_DISABLED")).unwrap_or_else(|_| "false".to_string()) != "true"
}
