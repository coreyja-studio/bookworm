use std::collections::HashMap;

use axum::{
    extract::{Form, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Redirect},
    routing::get,
};
use cja::{
    color_eyre::{
        self,
        eyre::{Context as _, eyre},
    },
    server::{cookies::CookieKey, run_server},
    setup::{setup_sentry, setup_tracing},
};
use maud::{DOCTYPE, Markup, PreEscaped, html};
use serde::Deserialize;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tracing::info;

pub use cja::Result;

#[derive(Clone)]
pub struct AppState {
    db: sqlx::PgPool,
    cookie_key: CookieKey,
}

impl AppState {
    async fn from_env() -> color_eyre::Result<Self> {
        let db = setup_db_pool().await?;
        let cookie_key = CookieKey::from_env_or_generate()?;
        Ok(Self { db, cookie_key })
    }
}

impl cja::app_state::AppState for AppState {
    fn version(&self) -> &'static str {
        "unknown"
    }

    fn db(&self) -> &sqlx::PgPool {
        &self.db
    }

    fn cookie_key(&self) -> &CookieKey {
        &self.cookie_key
    }
}

fn main() -> color_eyre::Result<()> {
    let _sentry_guard = setup_sentry();

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()?
        .block_on(async { run_application().await })
}

#[tracing::instrument(err)]
pub async fn setup_db_pool() -> cja::Result<PgPool> {
    const MIGRATION_LOCK_ID: i64 = 0xB0_0B_00_0B_00_0B_00;

    let database_url = std::env::var("DATABASE_URL").wrap_err("DATABASE_URL must be set")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    sqlx::query!("SELECT pg_advisory_lock($1)", MIGRATION_LOCK_ID)
        .execute(&pool)
        .await?;

    sqlx::migrate!().run(&pool).await?;

    let unlock_result = sqlx::query!("SELECT pg_advisory_unlock($1)", MIGRATION_LOCK_ID)
        .fetch_one(&pool)
        .await?
        .pg_advisory_unlock;

    match unlock_result {
        Some(true) => tracing::info!("Migration lock unlocked"),
        Some(false) => tracing::warn!("Failed to unlock migration lock"),
        None => return Err(eyre!("Failed to unlock migration lock")),
    }

    Ok(pool)
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

// ---------------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------------

pub fn routes(app_state: AppState) -> axum::Router {
    axum::Router::new()
        .route("/", get(home))
        .route("/log", get(log_form).post(log_read))
        .route("/history", get(history))
        .route("/stats", get(stats))
        .route("/manifest.webmanifest", get(manifest))
        .route("/icon.svg", get(icon_svg))
        .with_state(app_state)
}

async fn manifest() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/manifest+json")],
        include_str!("static/manifest.json"),
    )
}

async fn icon_svg() -> impl IntoResponse {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "image/svg+xml"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        include_str!("static/icon.svg"),
    )
}

// ---------------------------------------------------------------------------
// Handler structs
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct LogReadInput {
    title: String,
    author: String,
}

#[derive(Deserialize)]
struct HistoryParams {
    page: Option<u32>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn home() -> Redirect {
    Redirect::to("/stats")
}

async fn log_form(Query(params): Query<HashMap<String, String>>) -> Markup {
    let logged = params.get("logged").is_some_and(|v| v == "true");
    let content = html! {
        @if logged {
            div class="bg-green-100 border border-green-300 text-green-800 px-4 py-3 rounded-lg mb-6" {
                "Book logged successfully!"
            }
        }
        h1 class="text-3xl font-bold mb-6" { "Log a Read" }
        form method="post" action="/log" class="bg-linen rounded-2xl p-8 space-y-6 max-w-lg" {
            div {
                label for="title" class="block text-sm font-semibold mb-2" { "Title" }
                input type="text" name="title" id="title" required
                    class="w-full px-4 py-3 rounded-lg border border-spine/30 bg-parchment focus:outline-none focus:ring-2 focus:ring-gilded";
            }
            div {
                label for="author" class="block text-sm font-semibold mb-2" { "Author" }
                input type="text" name="author" id="author"
                    class="w-full px-4 py-3 rounded-lg border border-spine/30 bg-parchment focus:outline-none focus:ring-2 focus:ring-gilded";
            }
            button type="submit" class="bg-spine text-parchment px-6 py-3 rounded-lg font-semibold hover:bg-pressed transition-colors" {
                "Log Read"
            }
        }
    };
    layout("Log a Read", &content)
}

async fn log_read(State(state): State<AppState>, Form(input): Form<LogReadInput>) -> Redirect {
    let title = input.title.trim().to_string();
    let author = input.author.trim().to_string();

    if title.is_empty() {
        return Redirect::to("/log");
    }

    let book = sqlx::query_scalar!(
        r#"INSERT INTO books (title, author)
           VALUES ($1, $2)
           ON CONFLICT (title, author) DO UPDATE SET title = EXCLUDED.title
           RETURNING book_id"#,
        title,
        author
    )
    .fetch_one(&state.db)
    .await;

    match book {
        Ok(book_id) => {
            if let Err(e) = sqlx::query!("INSERT INTO reads (book_id) VALUES ($1)", book_id)
                .execute(&state.db)
                .await
            {
                tracing::error!("Failed to insert read: {e}");
                return Redirect::to("/log");
            }
            Redirect::to("/log?logged=true")
        }
        Err(e) => {
            tracing::error!("Failed to upsert book: {e}");
            Redirect::to("/log")
        }
    }
}

struct ReadEntry {
    title: String,
    author: String,
    read_date: chrono::NaiveDate,
}

async fn history(State(state): State<AppState>, Query(params): Query<HistoryParams>) -> Markup {
    let page = params.page.unwrap_or(0);
    let offset = i64::from(page) * 50;

    let rows = sqlx::query_as!(
        ReadEntry,
        r#"SELECT b.title, b.author, r.read_date
           FROM reads r
           JOIN books b ON b.book_id = r.book_id
           ORDER BY r.read_date DESC, r.created_at DESC
           LIMIT 50 OFFSET $1"#,
        offset
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_else(|e| {
        tracing::error!("Failed to fetch reading history: {e}");
        Vec::new()
    });

    let has_next = rows.len() == 50;
    let has_prev = page > 0;

    let content = html! {
        h1 class="text-3xl font-bold mb-6" { "Reading History" }
        @if rows.is_empty() {
            p class="text-spine/70" { "No reads yet. Go log some books!" }
        } @else {
            div class="space-y-3" {
                @for row in &rows {
                    div class="bg-linen rounded-xl p-4 flex justify-between items-center" {
                        div {
                            span class="font-semibold" { (row.title) }
                            @if !row.author.is_empty() {
                                span class="text-spine/70 ml-2" { "by " (row.author) }
                            }
                        }
                        span class="text-sm text-spine/60" { (row.read_date) }
                    }
                }
            }
        }
        div class="flex justify-between mt-8" {
            @if has_prev {
                a href=(format!("/history?page={}", page - 1)) class="text-spine hover:text-pressed font-semibold" { "Previous" }
            } @else {
                span {}
            }
            @if has_next {
                a href=(format!("/history?page={}", page + 1)) class="text-spine hover:text-pressed font-semibold" { "Next" }
            }
        }
    };
    layout("History", &content)
}

async fn stats(State(state): State<AppState>) -> Markup {
    let total_reads: i64 = sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM reads"#)
        .fetch_one(&state.db)
        .await
        .unwrap_or_else(|e| {
            tracing::error!("Failed to fetch total reads: {e}");
            0
        });

    let unique_books: i64 =
        sqlx::query_scalar!(r#"SELECT COUNT(DISTINCT book_id) as "count!" FROM reads"#)
            .fetch_one(&state.db)
            .await
            .unwrap_or_else(|e| {
                tracing::error!("Failed to fetch unique books: {e}");
                0
            });

    let reads_this_week: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!" FROM reads WHERE read_date >= CURRENT_DATE - INTERVAL '7 days'"#
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or_else(|e| {
        tracing::error!("Failed to fetch reads this week: {e}");
        0
    });

    let progress = std::cmp::min((unique_books * 100) / 1_000, 100);

    let content = html! {
        h1 class="text-3xl font-bold mb-8" { "Reading Stats" }
        div class="grid grid-cols-1 sm:grid-cols-3 gap-6 mb-8" {
            div class="bg-linen rounded-2xl p-6 text-center" {
                div class="text-4xl font-bold text-spine" { (total_reads) }
                div class="text-sm text-spine/70 mt-1" { "Total Reads" }
            }
            div class="bg-linen rounded-2xl p-6 text-center" {
                div class="text-4xl font-bold text-spine" { (unique_books) }
                div class="text-sm text-spine/70 mt-1" { "Unique Books" }
            }
            div class="bg-linen rounded-2xl p-6 text-center" {
                div class="text-4xl font-bold text-spine" { (reads_this_week) }
                div class="text-sm text-spine/70 mt-1" { "Reads This Week" }
            }
        }
        div class="bg-linen rounded-2xl p-6" {
            div class="flex justify-between mb-2" {
                span class="font-semibold" { "Progress to 1,000 Unique Books" }
                span class="text-spine/70" { (progress) "%" }
            }
            div class="w-full bg-parchment rounded-full h-4 overflow-hidden" {
                div class="bg-gilded h-4 rounded-full transition-all" style=(format!("width: {}%", progress)) {}
            }
            div class="text-sm text-spine/60 mt-2" {
                (unique_books) " / 1,000"
            }
        }
    };
    layout("Stats", &content)
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

fn layout(title: &str, content: &Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover";
                title { (title) " | Bookworm" }

                // PWA manifest
                link rel="manifest" href="/manifest.webmanifest";

                // Theme and branding
                meta name="theme-color" content="#8B4513";
                meta name="background-color" content="#FDF6EC";
                meta name="description" content="Track reads for the 1,000 Books Before Kindergarten challenge";

                // iOS PWA support
                meta name="apple-mobile-web-app-capable" content="yes";
                meta name="apple-mobile-web-app-status-bar-style" content="default";
                meta name="apple-mobile-web-app-title" content="Bookworm";
                link rel="apple-touch-icon" href="/icon.svg";

                // Favicon
                link rel="icon" href="/icon.svg" type="image/svg+xml";

                // Disable phone number detection
                meta name="format-detection" content="telephone=no";

                // Fonts and styles
                link rel="preconnect" href="https://fonts.googleapis.com";
                link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="";
                link href="https://fonts.googleapis.com/css2?family=Literata:opsz,wght@7..72,200;7..72,400;7..72,600;7..72,700&display=swap" rel="stylesheet";
                script src="https://cdn.tailwindcss.com" {}
                (PreEscaped(tailwind_config()))
            }
            body class="bg-parchment text-ink font-medium min-h-screen flex flex-col" style="font-family: 'Literata', serif;" {
                (nav_header())
                main class="flex-1 max-w-4xl mx-auto px-4 py-8 w-full" {
                    (content)
                }
                (footer())
            }
        }
    }
}

fn tailwind_config() -> &'static str {
    r"<script>
    tailwind.config = {
      theme: {
        extend: {
          colors: {
            parchment: '#FDF6EC',
            ink: '#2C1810',
            spine: '#8B4513',
            gilded: '#D4A843',
            linen: '#F5E6D0',
            pressed: '#6B3A2A',
          }
        }
      }
    }
    </script>"
}

fn nav_header() -> Markup {
    html! {
        nav class="bg-linen" {
            div class="max-w-4xl mx-auto px-4 py-4 flex items-center justify-between" {
                a href="/stats" class="text-xl font-bold tracking-tight text-spine hover:text-pressed transition-colors" {
                    "Bookworm"
                }
                div class="flex items-center gap-6 text-sm font-semibold" {
                    a href="/stats" class="text-spine hover:text-pressed transition-colors" { "Stats" }
                    a href="/log" class="text-spine hover:text-pressed transition-colors" { "Log a Read" }
                    a href="/history" class="text-spine hover:text-pressed transition-colors" { "History" }
                }
            }
        }
    }
}

fn footer() -> Markup {
    html! {
        footer class="mt-auto shrink-0 bg-linen" {
            div class="max-w-4xl mx-auto px-4 py-4 text-center text-sm text-spine/60" {
                "Bookworm - Tracking reads for Amelia"
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Application tasks
// ---------------------------------------------------------------------------

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
