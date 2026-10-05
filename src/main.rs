use bookworm::{AppState, cron_registry, observability, routes};
use cja::{
    color_eyre,
    setup::{TracingConfig, setup_sentry},
    tasks::{ShutdownBudget, Supervisor},
};
use std::time::Duration;
use tracing::info;

/// Bookworm runs no job worker, so the whole budget is exit grace: time for
/// in-flight HTTP requests and the active cron tick (the weekly email, sent
/// directly from the cron worker) to finish. Fly's `kill_timeout` is 60s; 30s
/// here plus the two 5s telemetry reports below leaves 20s of headroom.
const SHUTDOWN_BUDGET: ShutdownBudget = ShutdownBudget {
    job_drain: Duration::ZERO,
    exit_grace: Duration::from_secs(30),
};

fn main() -> color_eyre::Result<()> {
    let _sentry_guard = setup_sentry();
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()?
        .block_on(run_application())
}

async fn run_application() -> cja::Result<()> {
    let identity = eyes_subscriber::ProcessIdentity::new(observability::ROLE);
    let eyes_shutdown_handle = TracingConfig::new("bookworm")
        .process(identity.clone())
        .init()?;
    let result = run_services(identity).await;
    if let Err(error) = &result {
        tracing::error!(error = %format!("{error:#}"), "Bookworm stopped with an error");
    }
    if let Some(eyes) = eyes_shutdown_handle {
        info!("Flushing Eyes telemetry");
        match tokio::time::timeout(Duration::from_secs(5), eyes.shutdown()).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => eprintln!("Failed to flush Eyes telemetry: {error}"),
            Err(_) => eprintln!("Eyes telemetry flush timed out"),
        }
    }
    result
}

async fn run_services(identity: eyes_subscriber::ProcessIdentity) -> cja::Result<()> {
    let app_state = AppState::from_env().await?;
    let server_enabled = is_feature_enabled("SERVER");
    let cron_registry = is_feature_enabled("CRON").then(cron_registry);
    if !server_enabled && cron_registry.is_none() {
        info!("No application tasks enabled");
        return Ok(());
    }
    let manifest = observability::manifest(identity, cron_registry.as_ref())
        .map_err(|error| color_eyre::eyre::eyre!(error))?;
    // Registers SIGTERM/SIGINT now, so a signal during Eyes registration is not lost.
    let mut supervisor = Supervisor::new(SHUTDOWN_BUDGET)?;
    let heartbeat = observability::start(&manifest).await;
    spawn_application_tasks(&mut supervisor, &app_state, server_enabled, cron_registry);
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
    if let Some(cron_registry) = cron_registry {
        info!("Cron Enabled");
        supervisor.spawn(
            "Cron worker",
            bookworm::run_cron(app_state.clone(), cron_registry, shutdown),
        );
    } else {
        info!("Cron Disabled");
    }
}

fn is_feature_enabled(feature: &str) -> bool {
    std::env::var(format!("{feature}_DISABLED")).unwrap_or_else(|_| "false".to_string()) != "true"
}
