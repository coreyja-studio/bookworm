use std::collections::HashMap;

use axum::{
    extract::{Form, Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
use cja::{
    color_eyre::{
        self,
        eyre::{Context as _, eyre},
    },
    server::cookies::CookieKey,
};
use maud::{DOCTYPE, Markup, PreEscaped, html};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, postgres::PgPoolOptions};

pub use cja::Result;

#[derive(Clone)]
pub struct AppState {
    db: sqlx::PgPool,
    cookie_key: CookieKey,
    http_client: reqwest::Client,
}

impl AppState {
    pub async fn from_env() -> color_eyre::Result<Self> {
        let db = setup_db_pool().await?;
        let cookie_key = CookieKey::from_env_or_generate()?;
        let http_client = reqwest::Client::builder()
            .user_agent("Byte/1.0 (contact: corey@coreyja.com)")
            .build()?;
        Ok(Self {
            db,
            cookie_key,
            http_client,
        })
    }

    #[must_use]
    pub fn for_testing(db: sqlx::PgPool) -> Self {
        let cookie_key = CookieKey::from_env_or_generate()
            .expect("CookieKey should be available in test environment");
        let http_client = reqwest::Client::builder()
            .user_agent("Byte/1.0 (contact: corey@coreyja.com)")
            .build()
            .expect("reqwest client should build");
        Self {
            db,
            cookie_key,
            http_client,
        }
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

// ---------------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------------

pub fn routes(app_state: AppState) -> axum::Router {
    axum::Router::new()
        .route("/", get(home))
        .route("/log", get(log_form).post(log_read))
        .route("/library", get(library))
        .route("/log/reread", post(log_reread))
        .route("/library/reread", post(library_reread))
        .route("/library/delete", post(library_delete))
        .route("/history", get(history_redirect))
        .route("/progress", get(progress))
        .route("/stats", get(stats))
        .route("/api/isbn/{isbn}", get(isbn_lookup))
        .route("/books/{book_id}", get(book_detail))
        .route("/books/{book_id}/read-again", post(book_read_again))
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
    isbn: Option<String>,
    cover_url: Option<String>,
}

#[derive(Deserialize)]
struct LibraryParams {
    page: Option<u32>,
    q: Option<String>,
    reread: Option<String>,
    deleted: Option<String>,
}

#[derive(Deserialize)]
struct ProgressParams {
    mode: Option<String>,
}

#[allow(dead_code)]
struct LibraryEntry {
    book_id: uuid::Uuid,
    title: String,
    author: String,
    read_count: i64,
    last_read_date: chrono::NaiveDate,
    cover_url: Option<String>,
}

#[allow(dead_code)]
struct ReadEntry {
    book_id: uuid::Uuid,
    title: String,
    author: String,
    read_date: chrono::NaiveDate,
    cover_url: Option<String>,
}

struct FaveBook {
    title: String,
    author: String,
    read_count: i64,
}

struct FaveAuthor {
    author: String,
    book_count: i64,
}

#[derive(Deserialize)]
struct RereadInput {
    book_id: uuid::Uuid,
}

#[derive(Deserialize)]
struct DeleteInput {
    book_id: uuid::Uuid,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn home() -> Redirect {
    Redirect::to("/stats")
}

#[allow(clippy::too_many_lines)]
async fn log_form(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Markup {
    let logged = params.get("logged").is_some_and(|v| v == "true");

    let total_reads: i64 =
        sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM reads WHERE deleted_at IS NULL"#)
            .fetch_one(&state.db)
            .await
            .unwrap_or_else(|e| {
                tracing::error!("Failed: {e}");
                0
            });

    let unique_books: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(DISTINCT book_id) as "count!" FROM reads WHERE deleted_at IS NULL"#
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or_else(|e| {
        tracing::error!("Failed: {e}");
        0
    });

    let recent = sqlx::query_as!(
        ReadEntry,
        r#"SELECT b.book_id, b.title, b.author, r.read_date, b.cover_url as "cover_url?"
           FROM reads r JOIN books b ON b.book_id = r.book_id
           WHERE r.deleted_at IS NULL
           ORDER BY r.created_at DESC LIMIT 3"#
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let content = html! {
        @if logged {
            div class="toast fixed bottom-4 left-1/2 -translate-x-1/2 bg-accent-green text-white px-4 py-2 rounded-xl shadow-lg z-50" {
                "Book logged! 📖"
            }
        }
        div class="bg-white rounded-2xl border border-card-border p-6 shadow-sm" {
            div class="flex justify-between items-center mb-4" {
                h2 class="font-heading text-2xl font-bold" { "Log a Book 📖" }
                button type="button" id="scan-btn"
                    class="text-2xl hover:scale-110 transition-transform" { "📷" }
            }
            form method="post" action="/log" class="space-y-4" {
                div {
                    label for="title" class="block text-xs font-bold text-subtext uppercase tracking-wide mb-1" { "TITLE" }
                    input type="text" name="title" id="title" required
                        placeholder="e.g., Goodnight Moon 🌙"
                        class="w-full bg-accent-bg-orange rounded-xl px-4 py-3 border-none focus:ring-2 focus:ring-accent-orange focus:outline-none";
                }
                div {
                    label for="author" class="block text-xs font-bold text-subtext uppercase tracking-wide mb-1" { "AUTHOR" }
                    input type="text" name="author" id="author"
                        placeholder="e.g., Margaret Wise Brown"
                        class="w-full bg-accent-bg-orange rounded-xl px-4 py-3 border-none focus:ring-2 focus:ring-accent-orange focus:outline-none";
                }
                p class="text-accent-orange text-sm" { "Already logged this one? It'll count as a re-read!" }
                input type="hidden" name="isbn" id="isbn" value="";
                input type="hidden" name="cover_url" id="cover_url" value="";
                button type="submit"
                    class="bg-gradient-to-r from-accent-orange to-accent-red text-white font-bold py-3 px-6 rounded-xl shadow-lg hover:shadow-xl transition-shadow w-full" {
                    "Log Book #" (total_reads + 1)
                }
            }
        }

        @if !recent.is_empty() {
            div class="mt-6" {
                h3 class="font-heading text-lg font-bold mb-3" { "Recently Added" }
                div class="space-y-2" {
                    @for (i, entry) in recent.iter().enumerate() {
                        @let colors = ["red", "orange", "yellow", "green", "blue", "purple", "pink", "teal"];
                        @let color = colors[i % colors.len()];
                        div class="bg-white rounded-xl border border-card-border p-3 flex items-center gap-3" {
                            span class=(format!("bg-accent-{color} text-white rounded-full w-8 h-8 flex items-center justify-center text-xs font-bold shrink-0")) {
                                "#"
                            }
                            div class="flex-1 min-w-0" {
                                div class="font-bold truncate" { (entry.title) }
                                @if !entry.author.is_empty() {
                                    div class="text-subtext text-sm truncate" { "by " (entry.author) }
                                }
                            }
                            form method="post" action="/log/reread" {
                                input type="hidden" name="book_id" value=(entry.book_id);
                                button type="submit" class="text-accent-orange text-sm font-bold hover:underline shrink-0" { "Re-read" }
                            }
                        }
                    }
                }
            }
        }

        div id="scanner-modal" class="hidden fixed inset-0 bg-black/80 z-50 flex items-center justify-center p-4" {
            div class="bg-white rounded-2xl p-6 max-w-md w-full" {
                div class="flex justify-between items-center mb-4" {
                    h2 class="font-heading text-xl font-bold" { "Scan ISBN Barcode" }
                    button type="button" id="scan-close"
                        class="text-subtext hover:text-ink text-2xl font-bold" { "\u{00d7}" }
                }
                div id="scanner-container" class="w-full" style="min-height:300px" {}
                p id="scan-status" class="text-sm text-subtext mt-3 text-center" {
                    "Point your camera at the book's barcode"
                }
            }
        }
        script type="module" { (PreEscaped(include_str!("../ts/dist/scanner.js"))) }
    };
    layout("Log a Read", "add", &content, total_reads, unique_books)
}

async fn log_read(State(state): State<AppState>, Form(input): Form<LogReadInput>) -> Redirect {
    let title = input.title.trim().to_string();
    let author = input.author.trim().to_string();

    if title.is_empty() {
        return Redirect::to("/log");
    }

    let isbn = input
        .isbn
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let cover_url = input
        .cover_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let book = sqlx::query_scalar!(
        r#"INSERT INTO books (title, author, isbn, cover_url)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (title, author) DO UPDATE
             SET isbn = COALESCE(EXCLUDED.isbn, books.isbn),
                 cover_url = COALESCE(EXCLUDED.cover_url, books.cover_url)
           RETURNING book_id"#,
        title,
        author,
        isbn,
        cover_url,
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

async fn log_reread(State(state): State<AppState>, Form(input): Form<RereadInput>) -> Redirect {
    if let Err(e) = sqlx::query!("INSERT INTO reads (book_id) VALUES ($1)", input.book_id)
        .execute(&state.db)
        .await
    {
        tracing::error!("Failed to insert re-read: {e}");
    }
    Redirect::to("/log?logged=true")
}

#[allow(clippy::too_many_lines, clippy::cast_possible_wrap)]
async fn library(State(state): State<AppState>, Query(params): Query<LibraryParams>) -> Markup {
    let page = params.page.unwrap_or(0);
    let offset = i64::from(page) * 50;
    let search = params.q.clone();

    let rows = sqlx::query_as!(
        LibraryEntry,
        r#"SELECT b.book_id, b.title, b.author, b.cover_url as "cover_url?",
               COUNT(r.read_id) as "read_count!",
               MAX(r.read_date) as "last_read_date!"
           FROM books b
           JOIN reads r ON r.book_id = b.book_id
           WHERE r.deleted_at IS NULL
             AND ($1::TEXT IS NULL OR b.title ILIKE '%' || $1 || '%' OR b.author ILIKE '%' || $1 || '%')
           GROUP BY b.book_id, b.title, b.author, b.cover_url
           ORDER BY MAX(r.read_date) DESC, MAX(r.created_at) DESC
           LIMIT 50 OFFSET $2"#,
        search.as_deref(),
        offset
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_else(|e| {
        tracing::error!("Failed to fetch library: {e}");
        Vec::new()
    });

    let total_reads: i64 =
        sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM reads WHERE deleted_at IS NULL"#)
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

    let unique_books: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(DISTINCT book_id) as "count!" FROM reads WHERE deleted_at IS NULL"#
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let has_next = rows.len() == 50;
    let has_prev = page > 0;
    let colors = [
        "red", "orange", "yellow", "green", "blue", "purple", "pink", "teal",
    ];

    let content = html! {
        @if params.reread.is_some() {
            div class="toast fixed bottom-4 left-1/2 -translate-x-1/2 bg-accent-green text-white px-4 py-2 rounded-xl shadow-lg z-50" {
                "Re-read logged! 📖"
            }
        }
        @if params.deleted.is_some() {
            div class="toast fixed bottom-4 left-1/2 -translate-x-1/2 bg-accent-red text-white px-4 py-2 rounded-xl shadow-lg z-50" {
                "Book removed from library"
            }
        }

        form method="get" action="/library" class="mb-4" {
            input type="text" name="q" value=(params.q.as_deref().unwrap_or(""))
                placeholder="🔍 Search books..."
                class="w-full bg-white rounded-xl border border-card-border px-4 py-3 focus:ring-2 focus:ring-accent-orange focus:outline-none";
        }

        @if rows.is_empty() {
            div class="text-center text-subtext py-12" {
                "No books yet — go log some! 📚"
            }
        } @else {
            div class="space-y-3" {
                @for (i, row) in rows.iter().enumerate() {
                    @let color = colors[i % colors.len()];
                    div class="bg-white rounded-xl border border-card-border p-4" {
                        div class="flex items-start gap-3" {
                            @if let Some(cover) = &row.cover_url {
                                img src=(cover) alt=(row.title) class="w-10 h-14 object-cover rounded shrink-0 mt-0.5";
                            } @else {
                                span class=(format!("bg-accent-{color} text-white rounded-full w-8 h-8 flex items-center justify-center text-xs font-bold shrink-0 mt-0.5")) {
                                    "#" (offset + i as i64 + 1)
                                }
                            }
                            a href=(format!("/books/{}", row.book_id)) class="flex-1 min-w-0 block" {
                                div class="flex items-center gap-2" {
                                    span class="font-bold line-clamp-2" { (row.title) }
                                    @if row.read_count > 1 {
                                        span class="bg-accent-bg-purple text-accent-purple rounded-full px-2 py-0.5 text-xs font-bold shrink-0" {
                                            "×" (row.read_count)
                                        }
                                    }
                                }
                                @if !row.author.is_empty() {
                                    div class="text-subtext text-sm" { "by " (row.author) }
                                }
                                div class="text-subtext text-xs mt-1" { "Last read: " (row.last_read_date) }
                            }
                            div class="flex items-center gap-2 shrink-0" {
                                form method="post" action="/library/reread" {
                                    input type="hidden" name="book_id" value=(row.book_id);
                                    button type="submit" class="text-accent-orange text-sm font-bold hover:underline" { "Re-read" }
                                }
                                form method="post" action="/library/delete" onsubmit="return confirm('Remove this book from your library?')" {
                                    input type="hidden" name="book_id" value=(row.book_id);
                                    button type="submit" class="text-subtext hover:text-accent-red" { "🗑️" }
                                }
                            }
                        }
                    }
                }
            }
        }

        div class="flex justify-between mt-6" {
            @if has_prev {
                @let prev_q = params.q.as_deref().map_or(String::new(), |q| format!("&q={}", urlencoding::encode(q)));
                a href=(format!("/library?page={}{}", page - 1, prev_q)) class="text-accent-orange font-bold" { "← Previous" }
            } @else {
                span {}
            }
            @if has_next {
                @let next_q = params.q.as_deref().map_or(String::new(), |q| format!("&q={}", urlencoding::encode(q)));
                a href=(format!("/library?page={}{}", page + 1, next_q)) class="text-accent-orange font-bold" { "Next →" }
            }
        }
    };
    layout(
        "Amelia's Library",
        "library",
        &content,
        total_reads,
        unique_books,
    )
}

async fn library_reread(State(state): State<AppState>, Form(input): Form<RereadInput>) -> Redirect {
    if let Err(e) = sqlx::query!("INSERT INTO reads (book_id) VALUES ($1)", input.book_id)
        .execute(&state.db)
        .await
    {
        tracing::error!("Failed to insert re-read: {e}");
    }
    Redirect::to("/library?reread=true")
}

async fn library_delete(State(state): State<AppState>, Form(input): Form<DeleteInput>) -> Redirect {
    if let Err(e) = sqlx::query!(
        "UPDATE reads SET deleted_at = NOW() WHERE book_id = $1 AND deleted_at IS NULL",
        input.book_id
    )
    .execute(&state.db)
    .await
    {
        tracing::error!("Failed to soft-delete reads: {e}");
    }
    Redirect::to("/library?deleted=true")
}

async fn history_redirect() -> Redirect {
    Redirect::permanent("/library")
}

#[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
async fn progress(State(state): State<AppState>, Query(params): Query<ProgressParams>) -> Markup {
    let show_reads = params.mode.as_deref() == Some("reads");

    let total_reads: i64 =
        sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM reads WHERE deleted_at IS NULL"#)
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

    let unique_books: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(DISTINCT book_id) as "count!" FROM reads WHERE deleted_at IS NULL"#
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let progress_count = if show_reads {
        total_reads
    } else {
        unique_books
    };
    let progress_label = if show_reads {
        "total reads"
    } else {
        "unique books"
    };
    let percentage = std::cmp::min((progress_count * 100) / 1000, 100);
    let amelia_birthday = chrono::NaiveDate::from_ymd_opt(2025, 1, 18).unwrap();
    let kindergarten_start = chrono::NaiveDate::from_ymd_opt(2030, 9, 1).unwrap();
    let today = chrono::Utc::now().date_naive();
    let days_left = (kindergarten_start - today).num_days().max(0);
    let total_days = (kindergarten_start - amelia_birthday).num_days();
    let days_elapsed = (today - amelia_birthday).num_days().max(0);
    let timeline_pct = (days_elapsed as f64 / total_days as f64 * 100.0).min(100.0);
    let remaining = (1000 - progress_count).max(0);
    let months_left = days_left as f64 / 30.44;
    let per_month = if months_left > 0.0 {
        remaining as f64 / months_left
    } else {
        0.0
    };
    let per_week = if days_left > 0 {
        remaining as f64 / (days_left as f64 / 7.0)
    } else {
        0.0
    };

    let milestones: [(i64, &str, &str); 5] = [
        (100, "🌟", "red"),
        (250, "🔥", "orange"),
        (500, "🚀", "yellow"),
        (750, "💎", "blue"),
        (1000, "🏆", "purple"),
    ];

    let active_tab = "bg-ink text-white";
    let inactive_tab = "bg-white text-subtext hover:bg-gray-50";

    let content = html! {
        div class="flex justify-center mb-6" {
            div class="inline-flex rounded-full border border-card-border overflow-hidden text-sm font-bold" {
                a href="/progress"
                    class=(format!("px-4 py-2 transition-colors {}", if show_reads { inactive_tab } else { active_tab })) {
                    "Unique Books"
                }
                a href="/progress?mode=reads"
                    class=(format!("px-4 py-2 transition-colors {}", if show_reads { active_tab } else { inactive_tab })) {
                    "Total Reads"
                }
            }
        }

        div class="text-center mb-6" {
            div class="font-heading text-6xl font-extrabold text-accent-green" {
                (percentage) "%"
            }
            div class="text-subtext text-sm mt-1" {
                (progress_count) " of 1,000 " (progress_label)
            }
        }

        div class="bg-white rounded-2xl border border-card-border p-6 shadow-sm mb-6" {
            h3 class="font-heading text-lg font-bold mb-3" { "Countdown to Kindergarten" }
            div class="bg-accent-bg-blue rounded-full h-4 relative overflow-hidden" {
                div class="h-full rounded-full" style=(format!("width: {timeline_pct:.1}%; background: linear-gradient(to right, #FF6B6B, #FFa040, #FFD036, #5CD08E, #50B4F0, #A882F0)")) {}
                div class="absolute top-1/2 -translate-y-1/2 -translate-x-1/2 text-sm" style=(format!("left: {timeline_pct:.1}%")) { "📖" }
            }
            div class="flex justify-between mt-2 text-xs text-subtext" {
                span { "Jan 2025" }
                span { "Sep 2030" }
            }
        }

        div class="grid grid-cols-3 gap-3 mb-6" {
            div class="bg-white rounded-xl border border-card-border p-4 text-center" {
                div class="font-heading text-2xl font-bold text-accent-blue" { (days_left) }
                div class="text-xs text-subtext" { "Days Left" }
            }
            div class="bg-white rounded-xl border border-card-border p-4 text-center" {
                div class="font-heading text-2xl font-bold text-accent-orange" { (format!("{per_month:.1}")) }
                div class="text-xs text-subtext" { @if show_reads { "Reads/Month" } @else { "Books/Month" } }
            }
            div class="bg-white rounded-xl border border-card-border p-4 text-center" {
                div class="font-heading text-2xl font-bold text-accent-purple" { (format!("{per_week:.1}")) }
                div class="text-xs text-subtext" { @if show_reads { "Reads/Week" } @else { "Books/Week" } }
            }
        }

        div class="flex flex-wrap justify-center gap-3 mb-6" {
            @for (threshold, emoji, color) in &milestones {
                @let earned = progress_count >= *threshold;
                @let opacity = if earned { "" } else { " opacity-30" };
                div class=(format!("bg-accent-bg-{color} rounded-xl px-4 py-2 text-center{opacity}")) {
                    div class="text-2xl" { (*emoji) }
                    div class="text-xs font-bold text-ink" { (threshold) }
                }
            }
        }

        div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 gap-4 mt-6" {
            @for i in 0..10usize {
                (checkbox_grid(i, progress_count))
            }
        }
    };
    layout("Progress", "progress", &content, total_reads, unique_books)
}

#[allow(clippy::cast_possible_wrap)]
fn checkbox_grid(block_index: usize, unique_books: i64) -> Markup {
    let start = (block_index * 100) as i64;
    let filled = std::cmp::min((unique_books - start).max(0), 100);
    let is_complete = filled >= 100;
    let colors = [
        "red", "orange", "yellow", "green", "blue", "purple", "pink", "teal",
    ];
    let color = colors[block_index % colors.len()];

    let container_class = if is_complete {
        format!("rounded-xl p-3 border-2 border-accent-{color} bg-accent-bg-{color}")
    } else {
        "rounded-xl p-3 border-2 border-dashed border-card-border bg-white".to_string()
    };

    html! {
        div class=(container_class) {
            div class="flex justify-between items-center mb-2" {
                span class="text-xs font-bold text-subtext" {
                    (start + 1) "-" (start + 100)
                }
                @if is_complete { span { "⭐" } }
            }
            div class="grid grid-cols-10 gap-0.5" {
                @for i in 0..100i64 {
                    @let cell_class = if i < filled {
                        format!("w-full aspect-square rounded-sm bg-accent-{color}")
                    } else {
                        "w-full aspect-square rounded-sm bg-gray-100".to_string()
                    };
                    div class=(cell_class) {}
                }
            }
        }
    }
}

#[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
async fn stats(State(state): State<AppState>) -> Markup {
    let total_reads: i64 =
        sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM reads WHERE deleted_at IS NULL"#)
            .fetch_one(&state.db)
            .await
            .unwrap_or_else(|e| {
                tracing::error!("Failed to fetch total reads: {e}");
                0
            });

    let unique_books: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(DISTINCT book_id) as "count!" FROM reads WHERE deleted_at IS NULL"#
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or_else(|e| {
        tracing::error!("Failed to fetch unique books: {e}");
        0
    });

    let reads_this_week: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!" FROM reads WHERE read_date >= CURRENT_DATE - INTERVAL '7 days' AND deleted_at IS NULL"#
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or_else(|e| {
        tracing::error!("Failed to fetch reads this week: {e}");
        0
    });

    let fave_book = sqlx::query_as!(
        FaveBook,
        r#"SELECT b.title, b.author, COUNT(r.read_id) as "read_count!"
           FROM reads r JOIN books b ON b.book_id = r.book_id
           WHERE r.deleted_at IS NULL
           GROUP BY b.book_id, b.title, b.author
           ORDER BY COUNT(r.read_id) DESC, MAX(r.read_date) DESC
           LIMIT 1"#
    )
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let fave_author = sqlx::query_as!(
        FaveAuthor,
        r#"SELECT b.author, COUNT(DISTINCT b.book_id) as "book_count!"
           FROM reads r JOIN books b ON b.book_id = r.book_id
           WHERE r.deleted_at IS NULL AND b.author != ''
           GROUP BY b.author
           ORDER BY COUNT(DISTINCT b.book_id) DESC
           LIMIT 1"#
    )
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let reads_per_day = reads_this_week as f64 / 7.0;
    let total_rereads = (total_reads - unique_books).max(0);
    let milestones: [(i64, &str); 5] = [
        (100, "🌟"),
        (250, "🔥"),
        (500, "🚀"),
        (750, "💎"),
        (1000, "🏆"),
    ];
    let milestones_reached = milestones
        .iter()
        .filter(|(n, _)| unique_books >= *n)
        .count();
    let kindergarten_start = chrono::NaiveDate::from_ymd_opt(2030, 9, 1).unwrap();
    let today = chrono::Utc::now().date_naive();
    let days_left = (kindergarten_start - today).num_days().max(0);
    let books_remaining = (1000 - unique_books).max(0);
    let months_left = days_left as f64 / 30.44;
    let books_per_month_needed = if months_left > 0.0 {
        books_remaining as f64 / months_left
    } else {
        0.0
    };

    let content = html! {
        div class="text-center mb-8" {
            div class="font-heading text-7xl font-extrabold" style="background: linear-gradient(to right, #FF6B6B, #FFD036, #A882F0); -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text;" {
                (total_reads)
            }
            div class="text-subtext text-sm mt-1" { "Total Reads" }
        }

        div class="grid grid-cols-2 gap-4 mb-6" {
            div class="bg-white rounded-2xl border border-card-border p-6 text-center" {
                div class="font-heading text-3xl font-bold text-accent-blue" { (reads_this_week) }
                div class="text-subtext text-sm mt-1" { "This Week" }
            }
            div class="bg-white rounded-2xl border border-card-border p-6 text-center" {
                div class="font-heading text-3xl font-bold text-accent-orange" { (format!("{reads_per_day:.1}")) }
                div class="text-subtext text-sm mt-1" { "Per Day" }
            }
        }

        div class="bg-white rounded-2xl border border-card-border p-6 mb-4" {
            h3 class="font-heading text-lg font-bold mb-2" { "Amelia's Fave ❤️" }
            @if let Some(fave) = &fave_book {
                div class="font-bold" { (fave.title) }
                @if !fave.author.is_empty() {
                    div class="text-subtext text-sm" { "by " (fave.author) }
                }
                div class="text-subtext text-sm" { "❤️ Read " (fave.read_count) " times" }
            } @else {
                div class="text-subtext" { "Start reading to find a favorite!" }
            }
        }

        div class="bg-white rounded-2xl border border-card-border p-6 mb-4" {
            h3 class="font-heading text-lg font-bold mb-2" { "Favorite Author 📝" }
            @if let Some(author) = &fave_author {
                div class="font-bold" { (author.author) }
                div class="text-subtext text-sm" { (author.book_count) " books" }
            } @else {
                div class="text-subtext" { "No authors yet" }
            }
        }

        div class="grid grid-cols-2 gap-4 mb-6" {
            div class="bg-white rounded-2xl border border-card-border p-6 text-center" {
                div class="font-heading text-2xl font-bold text-accent-purple" {
                    (milestones_reached) "/5"
                }
                div class="text-subtext text-sm mt-1" { "Milestones" }
                div class="text-lg mt-1" {
                    @for (n, emoji) in &milestones {
                        @let opacity = if unique_books >= *n { "" } else { " opacity-30" };
                        span class=(format!("inline-block{opacity}")) { (*emoji) }
                    }
                }
            }
            div class="bg-white rounded-2xl border border-card-border p-6 text-center" {
                div class="font-heading text-2xl font-bold text-accent-teal" { (total_rereads) }
                div class="text-subtext text-sm mt-1" { "Re-reads" }
            }
        }

        div class={
            @let (bg, border) = if days_left == 0 && unique_books >= 1000 {
                ("bg-accent-bg-green", "border-accent-green")
            } else if days_left == 0 {
                ("bg-accent-bg-red", "border-accent-red")
            } else if books_per_month_needed < 15.0 {
                ("bg-accent-bg-green", "border-accent-green")
            } else if books_per_month_needed < 25.0 {
                ("bg-accent-bg-yellow", "border-accent-yellow")
            } else {
                ("bg-accent-bg-red", "border-accent-red")
            };
            (format!("{bg} rounded-2xl border {border} p-6 text-center"))
        } {
            @if days_left == 0 && unique_books >= 1000 {
                div class="font-heading text-xl font-bold" { "Goal reached! 🏆" }
            } @else if days_left == 0 {
                div class="font-heading text-xl font-bold" { "Goal period ended" }
            } @else if books_per_month_needed < 15.0 {
                div class="font-heading text-xl font-bold" { "On track! 🎉" }
                div class="text-subtext text-sm mt-1" { (format!("{books_per_month_needed:.1}")) " books/month needed" }
            } @else if books_per_month_needed < 25.0 {
                div class="font-heading text-xl font-bold" { "Keep it up! 📚" }
                div class="text-subtext text-sm mt-1" { (format!("{books_per_month_needed:.1}")) " books/month needed" }
            } @else {
                div class="font-heading text-xl font-bold" { "Time to read more! 🏃" }
                div class="text-subtext text-sm mt-1" { (format!("{books_per_month_needed:.1}")) " books/month needed" }
            }
        }
    };
    layout("Stats", "stats", &content, total_reads, unique_books)
}

// ---------------------------------------------------------------------------
// Book detail
// ---------------------------------------------------------------------------

#[allow(dead_code)]
struct BookInfo {
    title: String,
    author: String,
    isbn: Option<String>,
    cover_url: Option<String>,
}

struct BookReadDate {
    read_date: chrono::NaiveDate,
}

#[allow(clippy::too_many_lines)]
async fn book_detail(State(state): State<AppState>, Path(book_id): Path<uuid::Uuid>) -> Markup {
    let book = sqlx::query_as!(
        BookInfo,
        r#"SELECT title, author, isbn, cover_url FROM books WHERE book_id = $1"#,
        book_id
    )
    .fetch_optional(&state.db)
    .await
    .unwrap_or_else(|e| {
        tracing::error!("Failed to fetch book: {e}");
        None
    });

    let total_reads: i64 =
        sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM reads WHERE deleted_at IS NULL"#)
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

    let unique_books: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(DISTINCT book_id) as "count!" FROM reads WHERE deleted_at IS NULL"#
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let Some(book) = book else {
        let content = html! {
            div class="text-center py-12" {
                div class="text-4xl mb-4" { "📖" }
                h1 class="font-heading text-2xl font-bold mb-2" { "Book Not Found" }
                p class="text-subtext mb-4" { "This book doesn't exist in the library." }
                a href="/library" class="text-accent-orange font-bold hover:underline" { "← Back to Library" }
            }
        };
        return layout("Not Found", "library", &content, total_reads, unique_books);
    };

    let reads = sqlx::query_as!(
        BookReadDate,
        r#"SELECT read_date FROM reads
           WHERE book_id = $1 AND deleted_at IS NULL
           ORDER BY read_date DESC, created_at DESC"#,
        book_id
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_else(|e| {
        tracing::error!("Failed to fetch reads for book: {e}");
        Vec::new()
    });

    let read_count = reads.len();
    let first_read = reads.last().map(|r| r.read_date);
    let last_read = reads.first().map(|r| r.read_date);

    let content = html! {
        a href="/library" class="text-accent-orange font-bold text-sm hover:underline inline-block mb-4" {
            "← Back to Library"
        }

        div class="bg-white rounded-2xl border border-card-border p-6 shadow-sm mb-6" {
            div class="flex gap-4" {
                // Book cover
                div class="shrink-0" {
                    @if let Some(cover) = &book.cover_url {
                        img src=(cover) alt=(book.title)
                            class="w-24 h-36 object-cover rounded-xl shadow-sm";
                    } @else {
                        div class="w-24 h-36 bg-accent-bg-purple rounded-xl flex items-center justify-center" {
                            span class="text-4xl" { "📖" }
                        }
                    }
                }

                // Title and author
                div class="flex-1 min-w-0" {
                    h1 class="font-heading text-2xl font-bold leading-tight line-clamp-3" { (book.title) }
                    @if !book.author.is_empty() {
                        p class="text-subtext mt-1" { "by " (book.author) }
                    }
                    @if let Some(isbn) = &book.isbn {
                        p class="text-subtext text-xs mt-2" { "ISBN: " (isbn) }
                    }
                }
            }

            // Read Again button
            form method="post" action=(format!("/books/{}/read-again", book_id)) class="mt-4" {
                button type="submit"
                    class="bg-gradient-to-r from-accent-orange to-accent-red text-white font-bold py-3 px-6 rounded-xl shadow-lg hover:shadow-xl transition-shadow w-full" {
                    "Read Again 📖"
                }
            }
        }

        // Stats cards
        div class="grid grid-cols-3 gap-3 mb-6" {
            div class="bg-white rounded-xl border border-card-border p-4 text-center" {
                div class="font-heading text-2xl font-bold text-accent-purple" { (read_count) }
                div class="text-xs text-subtext" { "Times Read" }
            }
            div class="bg-white rounded-xl border border-card-border p-4 text-center" {
                @if let Some(first) = first_read {
                    div class="font-heading text-sm font-bold text-accent-green" { (first.format("%b %-d, %Y")) }
                } @else {
                    div class="font-heading text-sm font-bold text-subtext" { "—" }
                }
                div class="text-xs text-subtext" { "First Read" }
            }
            div class="bg-white rounded-xl border border-card-border p-4 text-center" {
                @if let Some(last) = last_read {
                    div class="font-heading text-sm font-bold text-accent-blue" { (last.format("%b %-d, %Y")) }
                } @else {
                    div class="font-heading text-sm font-bold text-subtext" { "—" }
                }
                div class="text-xs text-subtext" { "Last Read" }
            }
        }

        // Reading timeline
        @if !reads.is_empty() {
            div class="bg-white rounded-2xl border border-card-border p-6 shadow-sm" {
                h2 class="font-heading text-lg font-bold mb-4" { "Reading Timeline" }
                div class="space-y-3" {
                    @for (i, read) in reads.iter().enumerate() {
                        @let colors = ["red", "orange", "yellow", "green", "blue", "purple", "pink", "teal"];
                        @let color = colors[i % colors.len()];
                        div class="flex items-center gap-3" {
                            span class=(format!("bg-accent-{color} text-white rounded-full w-8 h-8 flex items-center justify-center text-xs font-bold shrink-0")) {
                                (read_count - i)
                            }
                            div class="flex-1" {
                                div class="font-bold text-sm" { (read.read_date.format("%B %-d, %Y")) }
                                @if i == reads.len() - 1 {
                                    span class="text-xs text-accent-green font-bold" { "First read! ⭐" }
                                } @else if i == 0 {
                                    span class="text-xs text-subtext" { "Most recent" }
                                }
                            }
                        }
                    }
                }
            }
        }
    };
    layout(&book.title, "library", &content, total_reads, unique_books)
}

async fn book_read_again(
    State(state): State<AppState>,
    Path(book_id): Path<uuid::Uuid>,
) -> Redirect {
    if let Err(e) = sqlx::query!("INSERT INTO reads (book_id) VALUES ($1)", book_id)
        .execute(&state.db)
        .await
    {
        tracing::error!("Failed to insert read: {e}");
    }
    Redirect::to(&format!("/books/{book_id}"))
}

// ---------------------------------------------------------------------------
// ISBN Lookup
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct IsbnResult {
    title: String,
    author: String,
    cover_url: Option<String>,
    isbn: String,
}

async fn isbn_lookup(
    State(state): State<AppState>,
    axum::extract::Path(isbn): axum::extract::Path<String>,
) -> std::result::Result<axum::Json<IsbnResult>, axum::http::StatusCode> {
    let isbn = isbn.trim().replace('-', "");
    if !isbn.chars().all(|c| c.is_ascii_digit() || c == 'X')
        || (isbn.len() != 10 && isbn.len() != 13)
    {
        return Err(axum::http::StatusCode::BAD_REQUEST);
    }

    if let Some(result) = lookup_open_library(&state.http_client, &isbn).await {
        return Ok(axum::Json(result));
    }

    if let Some(result) = lookup_google_books(&state.http_client, &isbn).await {
        return Ok(axum::Json(result));
    }

    Err(axum::http::StatusCode::NOT_FOUND)
}

async fn lookup_open_library(client: &reqwest::Client, isbn: &str) -> Option<IsbnResult> {
    let ol_url = format!("https://openlibrary.org/isbn/{isbn}.json");
    let ol_resp: serde_json::Value = client
        .get(&ol_url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .await
        .ok()?;

    let title = ol_resp["title"]
        .as_str()
        .unwrap_or("Unknown Title")
        .to_string();

    let author = {
        let edition_author = get_author_from_keys(client, &ol_resp["authors"]).await;
        if edition_author.is_empty() {
            if let Some(work_key) = ol_resp["works"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|w| w["key"].as_str())
            {
                let work_url = format!("https://openlibrary.org{work_key}.json");
                async {
                    let resp = client
                        .get(&work_url)
                        .timeout(std::time::Duration::from_secs(10))
                        .send()
                        .await
                        .ok()?;
                    let val: serde_json::Value = resp.json().await.ok()?;
                    let authors = val["authors"].as_array()?;
                    let first = authors.first()?;
                    let author_key = first["author"]["key"]
                        .as_str()
                        .or_else(|| first["key"].as_str())?;
                    let author_url = format!("https://openlibrary.org{author_key}.json");
                    let author_resp = client
                        .get(&author_url)
                        .timeout(std::time::Duration::from_secs(10))
                        .send()
                        .await
                        .ok()?;
                    let author_val: serde_json::Value = author_resp.json().await.ok()?;
                    author_val["name"].as_str().map(String::from)
                }
                .await
                .unwrap_or_default()
            } else {
                String::new()
            }
        } else {
            edition_author
        }
    };

    let cover_url = ol_resp["covers"]
        .as_array()
        .filter(|a| !a.is_empty())
        .map(|_| format!("https://covers.openlibrary.org/b/isbn/{isbn}-M.jpg"));

    Some(IsbnResult {
        title,
        author,
        cover_url,
        isbn: isbn.to_string(),
    })
}

async fn lookup_google_books(client: &reqwest::Client, isbn: &str) -> Option<IsbnResult> {
    let url = format!("https://www.googleapis.com/books/v1/volumes?q=isbn:{isbn}");
    let resp: serde_json::Value = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .await
        .ok()?;

    let item = resp["items"].as_array()?.first()?;
    let info = &item["volumeInfo"];

    let title = info["title"].as_str()?.to_string();

    let author = info["authors"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|a| a.as_str())
        .unwrap_or("")
        .to_string();

    let cover_url = info["imageLinks"]["thumbnail"]
        .as_str()
        .map(|u| u.replace("http://", "https://"));

    Some(IsbnResult {
        title,
        author,
        cover_url,
        isbn: isbn.to_string(),
    })
}

async fn get_author_from_keys(
    client: &reqwest::Client,
    authors_value: &serde_json::Value,
) -> String {
    let Some(authors) = authors_value.as_array() else {
        return String::new();
    };
    let Some(first) = authors.first() else {
        return String::new();
    };
    let Some(key) = first["key"].as_str() else {
        return String::new();
    };
    let author_url = format!("https://openlibrary.org{key}.json");
    let name = async {
        let resp = client
            .get(&author_url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .ok()?;
        let val: serde_json::Value = resp.json().await.ok()?;
        val["name"].as_str().map(String::from)
    }
    .await;
    name.unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

fn layout(
    title: &str,
    active_tab: &str,
    content: &Markup,
    total_reads: i64,
    unique_books: i64,
) -> Markup {
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
                meta name="theme-color" content="#3D2C1E";
                meta name="background-color" content="#FFF6EC";
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
                link href="https://fonts.googleapis.com/css2?family=Baloo+2:wght@700;800&family=Nunito:wght@600;700;800;900&display=swap" rel="stylesheet";
                script src="https://cdn.tailwindcss.com" {}
                (PreEscaped(tailwind_config()))
                script src="https://unpkg.com/html5-qrcode@2.3.8/html5-qrcode.min.js" {}
                style {
                    (PreEscaped("@keyframes slideUp { from { transform: translateY(100%); opacity: 0; } to { transform: translateY(0); opacity: 1; } }"))
                    (PreEscaped("@keyframes fadeOut { from { opacity: 1; } to { opacity: 0; } }"))
                    (PreEscaped(".toast { animation: slideUp 0.3s ease-out, fadeOut 0.3s ease-in 2.7s forwards; }"))
                }
            }
            body class="bg-cream text-ink font-body font-semibold min-h-screen flex flex-col" {
                (nav_header(active_tab, total_reads, unique_books))
                main class="flex-1 max-w-lg mx-auto px-4 py-6 w-full" {
                    (content)
                }
                (footer())
                // Hidden div for Tailwind JIT class discovery
                div class="hidden bg-accent-red bg-accent-orange bg-accent-yellow bg-accent-green bg-accent-blue bg-accent-purple bg-accent-pink bg-accent-teal bg-accent-bg-red bg-accent-bg-orange bg-accent-bg-yellow bg-accent-bg-green bg-accent-bg-blue bg-accent-bg-purple bg-accent-bg-pink bg-accent-bg-teal border-accent-red border-accent-orange border-accent-yellow border-accent-green border-accent-blue border-accent-purple border-accent-pink border-accent-teal text-accent-red text-accent-orange text-accent-yellow text-accent-green text-accent-blue text-accent-purple text-accent-pink text-accent-teal" {}
            }
        }
    }
}

fn tailwind_config() -> &'static str {
    r#"<script>
    tailwind.config = {
      theme: {
        extend: {
          colors: {
            cream: '#FFF6EC',
            'card-border': '#FFE4C8',
            ink: '#3D2C1E',
            subtext: '#9B7B62',
            accent: {
              red: '#FF6B6B',
              orange: '#FFa040',
              yellow: '#FFD036',
              green: '#5CD08E',
              blue: '#50B4F0',
              purple: '#A882F0',
              pink: '#FF82B8',
              teal: '#40D0C8',
            },
            'accent-bg': {
              red: '#FFF0F0',
              orange: '#FFF4E8',
              yellow: '#FFFBE8',
              green: '#EEFFF4',
              blue: '#EEF6FF',
              purple: '#F4EEFF',
              pink: '#FFF0F6',
              teal: '#EEFFFE',
            },
          },
          fontFamily: {
            heading: ['"Baloo 2"', 'cursive'],
            body: ['Nunito', 'sans-serif'],
          },
        }
      }
    }
    </script>"#
}

fn nav_header(active_tab: &str, total_reads: i64, unique_books: i64) -> Markup {
    let pct = std::cmp::min((unique_books * 100) / 1000, 100);
    let tabs = [
        ("add", "📚", "Add", "/log"),
        ("library", "📋", "Library", "/library"),
        ("progress", "🎯", "Progress", "/progress"),
        ("stats", "⭐", "Stats", "/stats"),
    ];

    html! {
        nav class="bg-white rounded-b-3xl shadow-sm pb-2" {
            div class="max-w-lg mx-auto px-4 pt-4" {
                div class="text-center text-xs font-bold text-subtext uppercase tracking-widest mb-1" {
                    "✨ 1,000 BOOKS BEFORE KINDERGARTEN ✨"
                }
                div class="font-heading text-4xl font-extrabold text-center" {
                    (rainbow_title())
                }
                div class="flex justify-between text-sm text-subtext mt-2" {
                    span { (total_reads) " reads" }
                    @if unique_books >= 1000 {
                        span { "Goal reached! 🎉" }
                    } @else {
                        span { (1000 - unique_books) " to go!" }
                    }
                }
                div class="bg-accent-bg-orange rounded-full h-3 mt-1 overflow-hidden" {
                    div class="h-full rounded-full" style=(format!("width: {pct}%; background: linear-gradient(to right, #FF6B6B, #FFa040, #FFD036, #5CD08E, #50B4F0, #A882F0)")) {}
                }
                div class="flex justify-center gap-1 mt-3" {
                    @for (name, emoji, label, href) in &tabs {
                        @let is_active = *name == active_tab;
                        @let tab_color = match *name {
                            "add" => "red",
                            "library" => "blue",
                            "progress" => "green",
                            "stats" => "yellow",
                            _ => "orange",
                        };
                        a href=(*href) class={
                            @if is_active {
                                (format!("rounded-xl border border-accent-{tab_color} bg-accent-bg-{tab_color} px-3 py-1 text-center"))
                            } @else {
                                "px-3 py-1 text-subtext text-center"
                            }
                        } {
                            div { (*emoji) }
                            div class="text-xs" { (*label) }
                        }
                    }
                }
            }
        }
    }
}

fn rainbow_title() -> Markup {
    let text = "Bookworm";
    let colors = [
        "#FF6B6B", "#FFa040", "#FFD036", "#5CD08E", "#50B4F0", "#A882F0", "#FF82B8", "#40D0C8",
    ];
    html! {
        @for (i, ch) in text.chars().enumerate() {
            span style=(format!("color: {}", colors[i % colors.len()])) { (ch) }
        }
        " 🐛"
    }
}

fn footer() -> Markup {
    html! {
        footer class="mt-auto shrink-0" {
            div class="max-w-lg mx-auto px-4 py-4 text-center text-sm text-subtext" {
                "Bookworm 🐛 — Tracking reads for Amelia"
            }
        }
    }
}
