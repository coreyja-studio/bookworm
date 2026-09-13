use bookworm::{AppState, cron_registry, observability, routes};
use cja::{
    color_eyre,
    server::run_server,
    setup::{TracingConfig, setup_sentry},
};
use tracing::info;

fn main() -> color_eyre::Result<()> {
    let _sentry_guard = setup_sentry();

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()?
        .block_on(async { run_application().await })
}

async fn run_application() -> cja::Result<()> {
    // Keep the Eyes shutdown handle alive for the lifetime of the app: dropping
    // it stops the Eyes telemetry exporter (configured via EYES_ORG_ID/EYES_APP_ID).
    let identity = eyes_subscriber::ProcessIdentity::new(observability::ROLE);
    let eyes_shutdown_handle = TracingConfig::new("bookworm")
        .process(identity.clone())
        .init()?;

    let app_state = AppState::from_env().await?;

    let shutdown_token = cja::jobs::CancellationToken::new();

    // Build the cron registry once so it can seed the Eyes boot manifest before
    // being handed off to the cron worker.
    let server_enabled = is_feature_enabled("SERVER");
    let cron_registry = is_feature_enabled("CRON").then(cron_registry);

    let manifest = observability::manifest(identity, cron_registry.as_ref())
        .map_err(|error| color_eyre::eyre::eyre!(error))?;
    cja::eyes_manifest::send_manifest(manifest);

    info!("Spawning application tasks");
    let futures =
        spawn_application_tasks(&app_state, server_enabled, cron_registry, &shutdown_token);

    let shutdown_handle = tokio::spawn(async move {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to create SIGTERM handler");
        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
            .expect("Failed to create SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM, initiating graceful shutdown");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT, initiating graceful shutdown");
            }
        }

        shutdown_token.cancel();
    });

    let result = futures::future::try_join_all(futures).await;

    shutdown_handle.abort();

    if let Some(eyes) = eyes_shutdown_handle {
        info!("Flushing Eyes telemetry");
        if let Err(err) = eyes.shutdown().await {
            eprintln!("Failed to flush Eyes telemetry on shutdown: {err}");
        }
    }

    result?;
    Ok(())
}

fn spawn_application_tasks(
    app_state: &AppState,
    server_enabled: bool,
    cron_registry: Option<cja::cron::CronRegistry<AppState>>,
    shutdown_token: &cja::jobs::CancellationToken,
) -> Vec<tokio::task::JoinHandle<std::result::Result<(), cja::color_eyre::Report>>> {
    let mut futures = vec![];

    if server_enabled {
        info!("Server Enabled");
        futures.push(tokio::spawn(run_server(routes(app_state.clone()))));
    } else {
        info!("Server Disabled");
    }

    if let Some(cron_registry) = cron_registry {
        info!("Cron Enabled");
        let app = app_state.clone();
        let token = shutdown_token.clone();
        futures.push(tokio::spawn(async move {
            bookworm::run_cron(app, cron_registry, token).await
        }));
    } else {
        info!("Cron Disabled");
    }

    info!("All application tasks spawned successfully");
    futures
}

fn is_feature_enabled(feature: &str) -> bool {
    let env_var_name = format!("{feature}_DISABLED");
    let value = std::env::var(&env_var_name).unwrap_or_else(|_| "false".to_string());
    value != "true"
}
