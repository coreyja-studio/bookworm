use bookworm::{AppState, routes};
use cja::{
    color_eyre,
    server::run_server,
    setup::{setup_sentry, setup_tracing},
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
    setup_tracing("bookworm")?;

    let app_state = AppState::from_env().await?;

    let shutdown_token = cja::jobs::CancellationToken::new();

    info!("Spawning application tasks");
    let futures = spawn_application_tasks(&app_state, &shutdown_token);

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

    result?;
    Ok(())
}

fn spawn_application_tasks(
    app_state: &AppState,
    #[allow(unused_variables)] shutdown_token: &cja::jobs::CancellationToken,
) -> Vec<tokio::task::JoinHandle<std::result::Result<(), cja::color_eyre::Report>>> {
    let mut futures = vec![];

    if is_feature_enabled("SERVER") {
        info!("Server Enabled");
        futures.push(tokio::spawn(run_server(routes(app_state.clone()))));
    } else {
        info!("Server Disabled");
    }

    info!("All application tasks spawned successfully");
    futures
}

fn is_feature_enabled(feature: &str) -> bool {
    let env_var_name = format!("{feature}_DISABLED");
    let value = std::env::var(&env_var_name).unwrap_or_else(|_| "false".to_string());
    value != "true"
}
