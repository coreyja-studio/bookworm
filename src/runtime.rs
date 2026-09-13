//! Supervise all enabled workers and drain them before the process exits.
use std::{collections::HashMap, future::Future, time::Duration};

use cja::{
    Result,
    color_eyre::eyre::{WrapErr, eyre},
    jobs::CancellationToken,
};
use tokio::task::{Id, JoinError, JoinSet};

#[derive(Default)]
pub struct ApplicationTasks {
    tasks: JoinSet<Result<()>>,
    names: HashMap<Id, &'static str>,
}

impl ApplicationTasks {
    pub fn spawn(
        &mut self,
        name: &'static str,
        task: impl Future<Output = Result<()>> + Send + 'static,
    ) {
        let handle = self.tasks.spawn(task);
        self.names.insert(handle.id(), name);
    }

    pub async fn supervise(mut self, shutdown: CancellationToken, grace: Duration) -> Result<()> {
        if self.tasks.is_empty() {
            return Ok(());
        }
        let mut result = tokio::select! {
            () = shutdown.cancelled() => Ok(()),
            outcome = self.tasks.join_next_with_id() => {
                self.completed(outcome.expect("tasks are present"), !shutdown.is_cancelled())
            }
        };
        shutdown.cancel();
        let drain = async {
            while let Some(outcome) = self.tasks.join_next_with_id().await {
                if let Err(error) = self.completed(outcome, false) {
                    tracing::error!(error = %format!("{error:#}"), "Application task failed during shutdown");
                    if result.is_ok() {
                        result = Err(error);
                    }
                }
            }
        };
        if tokio::time::timeout(grace, drain).await.is_err() {
            let mut pending: Vec<_> = self.names.values().copied().collect();
            pending.sort_unstable();
            tracing::error!(tasks = ?pending, "Application shutdown timed out");
            if result.is_ok() {
                result = Err(eyre!(
                    "Application shutdown timed out: {}",
                    pending.join(", ")
                ));
            }
            self.tasks.abort_all();
            while self.tasks.join_next().await.is_some() {}
        }
        result
    }

    fn completed(
        &mut self,
        outcome: std::result::Result<(Id, Result<()>), JoinError>,
        unexpected: bool,
    ) -> Result<()> {
        match outcome {
            Ok((id, result)) => {
                let name = self.names.remove(&id).unwrap_or("unknown");
                result.wrap_err_with(|| format!("{name} failed"))?;
                if unexpected {
                    Err(eyre!("{name} stopped unexpectedly"))
                } else {
                    Ok(())
                }
            }
            Err(error) => {
                let name = self.names.remove(&error.id()).unwrap_or("unknown");
                Err(error).wrap_err_with(|| format!("{name} task failed"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use tokio::sync::Notify;

    #[tokio::test]
    async fn worker_error_clean_exit_and_panic_cancel_and_drain_the_peer() {
        for mode in ["error", "exit", "panic"] {
            let shutdown = CancellationToken::new();
            let peer_token = shutdown.clone();
            let finished = Arc::new(AtomicBool::new(false));
            let peer_finished = finished.clone();
            let mut tasks = ApplicationTasks::default();
            tasks.spawn("server", async move {
                peer_token.cancelled().await;
                tokio::task::yield_now().await;
                peer_finished.store(true, Ordering::SeqCst);
                Ok(())
            });
            tasks.spawn("cron", async move {
                match mode {
                    "error" => Err(eyre!("database unavailable")),
                    "panic" => panic!("cron panic"),
                    _ => Ok(()),
                }
            });
            let error = tasks
                .supervise(shutdown.clone(), Duration::from_secs(1))
                .await
                .unwrap_err();
            let message = format!("{error:#}");
            assert!(message.contains("cron"), "{message}");
            assert!(
                message.contains(match mode {
                    "error" => "database unavailable",
                    "panic" => "cron panic",
                    _ => "stopped unexpectedly",
                }),
                "{message}"
            );
            assert!(shutdown.is_cancelled());
            assert!(finished.load(Ordering::SeqCst));
        }
    }

    #[tokio::test]
    async fn normal_shutdown_waits_for_active_work() {
        let shutdown = CancellationToken::new();
        let peer_token = shutdown.clone();
        let draining = Arc::new(Notify::new());
        let peer_draining = draining.clone();
        let release = Arc::new(Notify::new());
        let peer_release = release.clone();
        let mut tasks = ApplicationTasks::default();
        tasks.spawn("cron", async move {
            peer_token.cancelled().await;
            peer_draining.notify_one();
            peer_release.notified().await;
            Ok(())
        });
        let mut supervisor =
            tokio::spawn(tasks.supervise(shutdown.clone(), Duration::from_secs(2)));
        shutdown.cancel();
        tokio::time::timeout(Duration::from_secs(1), draining.notified())
            .await
            .unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut supervisor)
                .await
                .is_err()
        );
        release.notify_one();
        supervisor.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn an_error_during_normal_shutdown_is_reported() {
        let shutdown = CancellationToken::new();
        let peer_token = shutdown.clone();
        let mut tasks = ApplicationTasks::default();
        tasks.spawn("cron", async move {
            peer_token.cancelled().await;
            Err(eyre!("completion write failed"))
        });
        shutdown.cancel();
        let error = tasks
            .supervise(shutdown, Duration::from_secs(1))
            .await
            .unwrap_err();
        assert!(format!("{error:#}").contains("completion write failed"));
    }

    #[tokio::test]
    async fn shutdown_timeout_aborts_and_awaits_the_stuck_task() {
        struct Dropped(CancellationToken);
        impl Drop for Dropped {
            fn drop(&mut self) {
                self.0.cancel();
            }
        }
        let shutdown = CancellationToken::new();
        let dropped = CancellationToken::new();
        let guard = Dropped(dropped.clone());
        let mut tasks = ApplicationTasks::default();
        tasks.spawn("stuck cron", async move {
            let _guard = guard;
            std::future::pending().await
        });
        shutdown.cancel();
        let error = tasks
            .supervise(shutdown, Duration::from_millis(20))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("timed out: stuck cron"));
        assert!(
            dropped.is_cancelled(),
            "task must be destroyed before returning"
        );
    }

    #[tokio::test]
    async fn no_enabled_tasks_returns_immediately() {
        ApplicationTasks::default()
            .supervise(CancellationToken::new(), Duration::ZERO)
            .await
            .unwrap();
    }
}
