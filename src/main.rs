use bookworm::{AppState, cron_registry, observability, routes};
use cja::{
    color_eyre,
    jobs::CancellationToken,
    setup::{TracingConfig, setup_sentry},
};
use std::time::Duration;
use tracing::info;

mod runtime;

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
    let shutdown = CancellationToken::new();
    let signal_handle = spawn_signal_listener(shutdown.clone())?;
    let heartbeat = observability::start(&manifest).await;
    let tasks = spawn_application_tasks(&app_state, server_enabled, cron_registry, &shutdown);
    let result = tasks.supervise(shutdown, Duration::from_secs(30)).await;
    signal_handle.abort();
    let _ = signal_handle.await;
    if let Some(heartbeat) = heartbeat
        && let Err(error) = heartbeat.shutdown().await
    {
        tracing::warn!(%error, "Eyes process shutdown report failed");
    }
    result
}

fn spawn_signal_listener(shutdown: CancellationToken) -> cja::Result<tokio::task::JoinHandle<()>> {
    use tokio::signal::unix::{SignalKind, signal};
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;
    Ok(tokio::spawn(async move {
        tokio::select! {
            _ = sigterm.recv() => info!("Received SIGTERM, initiating graceful shutdown"),
            _ = sigint.recv() => info!("Received SIGINT, initiating graceful shutdown"),
        }
        shutdown.cancel();
    }))
}

fn spawn_application_tasks(
    app_state: &AppState,
    server_enabled: bool,
    cron_registry: Option<cja::cron::CronRegistry<AppState>>,
    shutdown: &CancellationToken,
) -> runtime::ApplicationTasks {
    let mut tasks = runtime::ApplicationTasks::default();
    if server_enabled {
        info!("Server Enabled");
        tasks.spawn(
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
        tasks.spawn(
            "Cron worker",
            bookworm::run_cron(app_state.clone(), cron_registry, shutdown.clone()),
        );
    } else {
        info!("Cron Disabled");
    }
    tasks
}

fn is_feature_enabled(feature: &str) -> bool {
    std::env::var(format!("{feature}_DISABLED")).unwrap_or_else(|_| "false".to_string()) != "true"
}
