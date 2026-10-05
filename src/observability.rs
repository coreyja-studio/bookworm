//! Passive operational evidence for Bookworm's requests and weekly email.
use crate::{AppState, Jobs};
use eyes_subscriber::{
    AggregateFunction as Agg, AppManifest, DashboardItem as Item, DashboardSection as Section,
    ExpectedProcessRole, NamedDashboard, NamedMetric, NamedMetricBuilder as Metric,
    ProcessHeartbeat, ProcessHeartbeatConfig, ProcessHeartbeatHandle, ProcessIdentity,
};
use std::time::Duration;

pub const ROLE: &str = "bookworm";
pub const CRON_TIMEZONE: cja::chrono_tz::Tz = cja::chrono_tz::America::New_York;

#[must_use]
pub fn git_sha() -> Option<&'static str> {
    option_env!("GIT_SHA").filter(|value| !value.is_empty())
}

pub fn manifest(
    identity: ProcessIdentity,
    crons: Option<&cja::cron::CronRegistry<AppState>>,
) -> Result<AppManifest, String> {
    let mut manifest = cja::eyes_manifest::build_boot_manifest::<Jobs, AppState>(
        Some(env!("CARGO_PKG_VERSION")),
        git_sha(),
        crons,
    );
    for cron in &mut manifest.crons {
        cron.schedule = format!("CRON_TZ={CRON_TIMEZONE} {}", cron.schedule);
    }
    let mut metrics = request_metrics()?;
    metrics.extend(email_metrics()?);
    Ok(manifest
        .process(identity)
        .expected_process_roles(vec![ExpectedProcessRole::new(ROLE).min_instances(0)])
        .monitors(vec![])
        .metrics(metrics)
        .dashboards(vec![dashboard()?]))
}

/// Register before heartbeats. Observability failures never prevent app startup.
pub async fn start(manifest: &AppManifest) -> Option<ProcessHeartbeatHandle> {
    let (Ok(org), Ok(app)) = (std::env::var("EYES_ORG_ID"), std::env::var("EYES_APP_ID")) else {
        return None;
    };
    let register = async {
        let org = org.parse()?;
        let app = app.parse()?;
        let base = std::env::var("EYES_URL")
            .unwrap_or_else(|_| "https://eyes.coreyja.com".into())
            .parse()?;
        eyes_subscriber::send_manifest_from_env(manifest).await?;
        Ok::<_, color_eyre::Report>(ProcessHeartbeat::spawn(
            ProcessHeartbeatConfig::from_manifest(base, org, app, manifest)?,
        )?)
    };
    match tokio::time::timeout(Duration::from_secs(10), register).await {
        Ok(Ok(handle)) => Some(handle),
        Ok(Err(error)) => {
            tracing::warn!(error = %format!("{error:#}"), "Eyes process registration failed");
            None
        }
        Err(_) => {
            tracing::warn!("Eyes process registration timed out");
            None
        }
    }
}

fn count(id: &str, title: &str, path: &str, value: &str) -> Result<Metric, String> {
    Metric::new(id, Agg::Count, None)?
        .display_name(title)
        .filter_eq(path, value)
}

fn request_metrics() -> Result<Vec<NamedMetric>, String> {
    Ok(vec![
        count("http.requests", "Requests", "semantic_kind", "http.request")?
            .unit("requests")
            .build()?,
        count(
            "http.requests.series",
            "Request volume",
            "semantic_kind",
            "http.request",
        )?
        .unit("requests")
        .time_bucket(3600)
        .build()?,
        Metric::new("http.latency.p95", Agg::P95, Some("duration"))?
            .filter_eq("semantic_kind", "http.request")?
            .display_name("Request latency · p95")
            .unit("µs")
            .build()?,
        count(
            "http.routes",
            "Requests by route",
            "semantic_kind",
            "http.request",
        )?
        .group_by("fields[\"http.route\"]")?
        .unit("requests")
        .build()?,
        count("errors", "Error events", "level", "ERROR")?
            .unit("events")
            .build()?,
        count("errors.series", "Error events", "level", "ERROR")?
            .unit("events")
            .time_bucket(3600)
            .build()?,
    ])
}

fn email_metrics() -> Result<Vec<NamedMetric>, String> {
    let mut metrics = vec![
        count(
            "cron.fires",
            "Weekly cron triggers",
            "semantic_kind",
            "cron.fire",
        )?
        .filter_eq("fields.task_name", "weekly_reading_email")?
        .unit("triggers")
        .build()?,
    ];
    for (id, title, event, unit) in [
        (
            "email.started",
            "Weekly email attempts",
            "bookworm.email_started",
            "attempts",
        ),
        (
            "email.skipped",
            "Skipped: no reads",
            "bookworm.email_skipped",
            "attempts",
        ),
        (
            "email.accepted",
            "SMTP sends accepted",
            "bookworm.smtp_accepted",
            "emails",
        ),
        (
            "email.completed",
            "Completed email attempts",
            "bookworm.email_completed",
            "attempts",
        ),
        (
            "email.failed",
            "Failed email attempts",
            "bookworm.email_failed",
            "attempts",
        ),
    ] {
        metrics.push(
            count(id, title, "fields.event_type", event)?
                .unit(unit)
                .build()?,
        );
    }
    Ok(metrics)
}

fn dashboard() -> Result<NamedDashboard, String> {
    Ok(NamedDashboard::new("bookworm-operations", "Bookworm operations")?
        .description("Real requests and weekly reading emails. Quiet weekdays are normal. Email runs Sunday at 6 p.m. Eastern; SMTP acceptance does not prove inbox delivery.")
        .default_range_seconds(604_800)
        .section(Section::new().title("At a glance")
            .item(Item::stat("http.requests")).item(Item::stat("http.latency.p95"))
            .item(Item::stat("errors")))
        .section(Section::new().title("Weekly reading email")
            .item(Item::stat("cron.fires")).item(Item::stat("email.started"))
            .item(Item::stat("email.skipped")).item(Item::stat("email.accepted"))
            .item(Item::stat("email.completed")).item(Item::stat("email.failed")))
        .section(Section::new().title("Traffic and errors")
            .item(Item::time_series("http.requests.series"))
            .item(Item::time_series("errors.series"))
            .item(Item::table("http.routes"))))
}
