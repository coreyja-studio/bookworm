use std::collections::HashMap;

use axum::{
    extract::{Form, Query, State},
    response::Redirect,
    routing::get,
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
        .route("/history", get(history))
        .route("/stats", get(stats))
        .route("/api/isbn/{isbn}", get(isbn_lookup))
        .with_state(app_state)
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
            input type="hidden" name="isbn" id="isbn" value="";
            input type="hidden" name="cover_url" id="cover_url" value="";
            div {
                button type="button" id="scan-btn"
                    class="bg-gilded text-ink px-6 py-3 rounded-lg font-semibold hover:bg-gilded/80 transition-colors w-full" {
                    "Scan Barcode"
                }
            }
            button type="submit" class="bg-spine text-parchment px-6 py-3 rounded-lg font-semibold hover:bg-pressed transition-colors" {
                "Log Read"
            }
        }
        div id="scanner-modal" class="hidden fixed inset-0 bg-ink/80 z-50 flex items-center justify-center p-4" {
            div class="bg-parchment rounded-2xl p-6 max-w-md w-full" {
                div class="flex justify-between items-center mb-4" {
                    h2 class="text-xl font-bold" { "Scan ISBN Barcode" }
                    button type="button" id="scan-close"
                        class="text-spine hover:text-pressed text-2xl font-bold" { "\u{00d7}" }
                }
                div id="scanner-container" class="w-full" style="min-height:300px" {}
                p id="scan-status" class="text-sm text-spine/70 mt-3 text-center" {
                    "Point your camera at the book's barcode"
                }
            }
        }
        script { (PreEscaped(include_str!("../ts/dist/scanner.js"))) }
    };
    layout("Log a Read", &content)
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

struct ReadEntry {
    title: String,
    author: String,
    read_date: chrono::NaiveDate,
    cover_url: Option<String>,
}

async fn history(State(state): State<AppState>, Query(params): Query<HistoryParams>) -> Markup {
    let page = params.page.unwrap_or(0);
    let offset = i64::from(page) * 50;

    let rows = sqlx::query_as!(
        ReadEntry,
        r#"SELECT b.title, b.author, r.read_date, b.cover_url as "cover_url?"
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
                    div class="bg-linen rounded-xl p-4 flex items-center gap-4" {
                        @if let Some(url) = &row.cover_url {
                            img src=(url) alt="" class="w-12 h-16 object-cover rounded shadow-sm" loading="lazy";
                        } @else {
                            div class="w-12 h-16 bg-spine/10 rounded flex items-center justify-center text-spine/40 text-xs" {
                                "No cover"
                            }
                        }
                        div class="flex-1" {
                            span class="font-semibold" { (row.title) }
                            @if !row.author.is_empty() {
                                span class="text-spine/70 ml-2" { "by " (row.author) }
                            }
                        }
                        span class="text-sm text-spine/60 shrink-0" { (row.read_date) }
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

    let ol_url = format!("https://openlibrary.org/isbn/{isbn}.json");
    let ol_resp: serde_json::Value = state
        .http_client
        .get(&ol_url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?
        .error_for_status()
        .map_err(|_| axum::http::StatusCode::NOT_FOUND)?
        .json()
        .await
        .map_err(|_| axum::http::StatusCode::BAD_GATEWAY)?;

    let title = ol_resp["title"]
        .as_str()
        .unwrap_or("Unknown Title")
        .to_string();

    let author = {
        let edition_author = get_author_from_keys(&state.http_client, &ol_resp["authors"]).await;
        if edition_author.is_empty() {
            if let Some(work_key) = ol_resp["works"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|w| w["key"].as_str())
            {
                let work_url = format!("https://openlibrary.org{work_key}.json");
                async {
                    let resp = state
                        .http_client
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
                    let author_resp = state
                        .http_client
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

    Ok(axum::Json(IsbnResult {
        title,
        author,
        cover_url,
        isbn,
    }))
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

fn layout(title: &str, content: &Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) " | Bookworm" }
                link rel="preconnect" href="https://fonts.googleapis.com";
                link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="";
                link href="https://fonts.googleapis.com/css2?family=Literata:opsz,wght@7..72,200;7..72,400;7..72,600;7..72,700&display=swap" rel="stylesheet";
                script src="https://cdn.tailwindcss.com" {}
                (PreEscaped(tailwind_config()))
                script src="https://unpkg.com/html5-qrcode@2.3.8/html5-qrcode.min.js" {}
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
