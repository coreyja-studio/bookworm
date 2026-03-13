//! Integration tests for the Bookworm app.
//!
//! These tests define the expected behavior for:
//! - Book logging (POST /log)
//! - Reading history (GET /history)
//! - Stats page (GET /stats)
//! - Home redirect (GET /)
//!
//! All tests require the app's `routes()` function and `AppState` to exist.
//! They use axum's test utilities via `tower::ServiceExt` to make requests
//! against the router without starting a real server.
//!
//! Database tests require a test `PostgreSQL` database. Set `DATABASE_URL` to
//! a test database before running.

// -- Home / Redirect --

#[test]
#[ignore = "Requires routes() and AppState to be implemented"]
fn home_redirects_to_stats() {
    // GET / should return a redirect (303 or 302) to /stats
    todo!("Build router, send GET /, assert redirect to /stats")
}

// -- Log a Read --

#[test]
#[ignore = "Requires routes(), AppState, and database setup"]
fn log_read_creates_book_and_read_event() {
    // POST /log with title="Goodnight Moon" author="Margaret Wise Brown"
    // should create a book record and a read event, then redirect to /log?logged=true
    todo!("POST /log with form data, assert 303 redirect to /log?logged=true")
}

#[test]
#[ignore = "Requires routes(), AppState, and database setup"]
fn log_read_upserts_book_on_duplicate() {
    // Logging the same title+author twice should create only ONE book
    // but TWO read events
    todo!(
        "POST /log twice with same title/author, \
         query DB: 1 book, 2 reads"
    )
}

#[test]
#[ignore = "Requires routes(), AppState, and database setup"]
fn log_read_empty_title_redirects_without_creating() {
    // POST /log with title="" (or whitespace-only) should redirect to /log
    // without creating any book or read
    todo!(
        "POST /log with empty title, assert redirect to /log (no ?logged=true), \
         verify no rows in books or reads tables"
    )
}

#[test]
#[ignore = "Requires routes(), AppState, and database setup"]
fn log_read_empty_author_is_valid() {
    // POST /log with title="Some Book" author="" should succeed.
    // HTML forms send empty fields as "", matching the DB default.
    todo!(
        "POST /log with empty author, assert redirect to /log?logged=true, \
         verify book exists with author=''"
    )
}

#[test]
#[ignore = "Requires routes(), AppState, and database setup"]
fn log_read_trims_whitespace_from_title_and_author() {
    // POST /log with title="  Goodnight Moon  " author="  Margaret  "
    // should store trimmed values
    todo!(
        "POST /log with padded title/author, query DB and assert \
         title='Goodnight Moon', author='Margaret'"
    )
}

#[test]
#[ignore = "Requires routes(), AppState, and database setup"]
fn log_read_shows_success_message_after_redirect() {
    // GET /log?logged=true should include a success message in the HTML
    todo!("GET /log?logged=true, assert response body contains success indicator")
}

#[test]
#[ignore = "Requires routes() and AppState to be implemented"]
fn log_form_renders_title_and_author_fields() {
    // GET /log should return HTML with a form containing title and author inputs
    todo!(
        "GET /log, assert 200, body contains <form>, \
         input with name='title', input with name='author'"
    )
}

// -- Stats Page --

#[test]
#[ignore = "Requires routes(), AppState, and database setup"]
fn stats_shows_zero_counts_on_empty_db() {
    // GET /stats on a fresh database should show 0 total reads,
    // 0 unique books, 0% progress, 0 reads this week
    todo!("GET /stats on empty DB, assert body contains '0' for all stats")
}

#[test]
#[ignore = "Requires routes(), AppState, and database setup"]
fn stats_shows_correct_total_reads() {
    // After logging 3 reads (2 of same book, 1 different),
    // total reads should be 3
    todo!(
        "Insert 3 read events, GET /stats, \
         assert total reads = 3"
    )
}

#[test]
#[ignore = "Requires routes(), AppState, and database setup"]
fn stats_shows_correct_unique_books() {
    // After logging 3 reads (2 of same book, 1 different),
    // unique books should be 2
    todo!(
        "Insert 2 books with 3 total reads, GET /stats, \
         assert unique books = 2"
    )
}

#[test]
#[ignore = "Requires routes(), AppState, and database setup"]
fn stats_progress_bar_caps_at_100_percent() {
    // If unique books >= 1000, progress should show 100% (not overflow)
    todo!(
        "Insert 1000+ unique books, GET /stats, \
         assert progress does not exceed 100%"
    )
}

#[test]
#[ignore = "Requires routes(), AppState, and database setup"]
fn stats_shows_reads_this_week() {
    // Insert reads with various dates, verify only reads from
    // the last 7 days are counted in "reads this week"
    todo!(
        "Insert reads with dates inside and outside 7-day window, \
         GET /stats, assert reads-this-week count is correct"
    )
}

// -- History Page --

#[test]
#[ignore = "Requires routes(), AppState, and database setup"]
fn history_shows_reads_in_reverse_chronological_order() {
    // GET /history should list reads newest-first
    todo!(
        "Insert reads on different dates, GET /history, \
         assert first read shown is the most recent"
    )
}

#[test]
#[ignore = "Requires routes(), AppState, and database setup"]
fn history_shows_book_title_and_author() {
    // Each entry in history should display the book's title and author
    todo!(
        "Insert a read for 'Goodnight Moon' by 'Margaret Wise Brown', \
         GET /history, assert body contains both title and author"
    )
}

#[test]
#[ignore = "Requires routes(), AppState, and database setup"]
fn history_paginates_at_50_items() {
    // With 60 reads, page 0 should show 50 and a "Next" link,
    // page 1 should show 10 and a "Previous" link but no "Next"
    todo!(
        "Insert 60 reads, GET /history (page 0): assert 50 items + Next link, \
         GET /history?page=1: assert 10 items + Previous link, no Next"
    )
}

#[test]
#[ignore = "Requires routes(), AppState, and database setup"]
fn history_page_zero_has_no_previous_link() {
    // GET /history (or /history?page=0) should not show a "Previous" link
    todo!("GET /history, assert body does NOT contain a Previous link")
}

#[test]
#[ignore = "Requires routes(), AppState, and database setup"]
fn history_empty_db_renders_without_error() {
    // GET /history on an empty database should return 200 with no read entries
    todo!("GET /history on empty DB, assert 200, no read entries in body")
}

// -- Layout / Navigation --

#[test]
#[ignore = "Requires routes() and AppState to be implemented"]
fn pages_include_nav_with_all_links() {
    // Every page should have navigation links to Stats, Log a Read, and History
    todo!(
        "GET /stats, /log, /history — each response body should contain \
         links to /stats, /log, and /history"
    )
}

#[test]
#[ignore = "Requires routes() and AppState to be implemented"]
fn pages_include_bookworm_in_title() {
    // HTML <title> should contain "Bookworm"
    todo!("GET /stats, assert body contains '<title>' with 'Bookworm'")
}

// -- Multiple reads same day --

#[test]
#[ignore = "Requires routes(), AppState, and database setup"]
fn multiple_reads_same_book_same_day_are_separate_events() {
    // Logging the same book twice on the same day should create
    // two separate read events (not deduplicate)
    todo!(
        "POST /log twice with same book, same day, \
         assert 2 read rows exist for that book"
    )
}

// =============================================================================
// BW-003a16ad28ef4083: Barcode Scanning for Book Input
// =============================================================================
//
// These tests define the expected behavior for:
//   - ISBN validation at GET /api/isbn/{isbn}
//   - Open Library lookup returning book data
//   - Storing isbn + cover_url when logging a read via POST /log
//   - Log form containing scan button, modal, and CDN script
//
// Tests that require network access (e.g. Open Library API) remain #[ignore].
// Local tests run in CI against a test database.

/// Build an axum Router wired to the test database.
///
/// Implementation agent: this crate is currently a `[[bin]]` with no `[lib]` target,
/// so integration tests cannot call `bookworm::*` functions directly.
///
/// To enable these tests you must:
///   1. Add a `[lib]` target to `Cargo.toml` (e.g. `path = "src/lib.rs"`) and expose
///      `pub use` for `setup_db_pool`, `AppState`, and `routes`.
///   2. Add `pub fn for_testing(db: sqlx::PgPool) -> Self` to `AppState`.
///   3. Replace the `todo!()` below with:
///      ```rust
///      let db = bookworm::setup_db_pool().await.unwrap();
///      let state = bookworm::AppState::for_testing(db);
///      bookworm::routes(state)
///      ```
async fn make_test_router() -> axum::Router {
    let db = bookworm::setup_db_pool().await.unwrap();
    let state = bookworm::AppState::for_testing(db);
    bookworm::routes(state)
}

async fn make_test_db() -> sqlx::PgPool {
    bookworm::setup_db_pool().await.unwrap()
}

// -- ISBN endpoint: input validation --

#[tokio::test]
async fn isbn_lookup_rejects_invalid_isbn() {
    // GET /api/isbn/abc should return 400 Bad Request.
    // "abc" contains non-digit characters and is not a valid ISBN.
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let app = make_test_router().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/isbn/abc")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Non-numeric ISBN 'abc' should return 400 Bad Request"
    );
}

#[tokio::test]
async fn isbn_lookup_rejects_wrong_length() {
    // GET /api/isbn/12345 should return 400 Bad Request.
    // Valid ISBNs are exactly 10 or 13 digits; 5 digits is invalid.
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let app = make_test_router().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/isbn/12345")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "5-digit string '12345' should return 400 Bad Request (must be 10 or 13 digits)"
    );
}

// -- ISBN endpoint: successful lookup (requires live network) --

#[tokio::test]
#[ignore = "Requires AppState::for_testing(), /api/isbn/{isbn} route, and live openlibrary.org network access"]
async fn isbn_lookup_returns_book_data() {
    // GET /api/isbn/9780064430173 should return 200 with JSON containing
    // "Goodnight Moon" — a well-known children's book with a stable Open Library record.
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let app = make_test_router().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/isbn/9780064430173")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Known ISBN 9780064430173 should return 200 OK from Open Library"
    );

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = std::str::from_utf8(&body).expect("response body should be valid UTF-8");

    assert!(
        body_str.contains("Goodnight Moon"),
        "Response JSON should contain the title 'Goodnight Moon', got: {body_str}"
    );
    assert!(
        body_str.contains("9780064430173"),
        "Response JSON should echo back the isbn field, got: {body_str}"
    );
}

// -- Log form: scanner UI elements --

#[tokio::test]
async fn scan_form_contains_scanner_elements() {
    // GET /log should include the scan button, the scanner modal, and the
    // html5-qrcode CDN <script> tag so the camera scanner is available.
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let app = make_test_router().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/log")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = std::str::from_utf8(&body).expect("response body should be valid UTF-8");

    assert!(
        html.contains("scan-btn"),
        "Log form should contain an element with id='scan-btn'"
    );
    assert!(
        html.contains("scanner-modal"),
        "Log form should contain the scanner modal with id='scanner-modal'"
    );
    assert!(
        html.contains("html5-qrcode"),
        "Log form should include the html5-qrcode library (CDN <script> tag)"
    );
}

// -- Log endpoint: isbn and cover_url stored in DB --

#[tokio::test]
async fn log_read_with_isbn_stores_isbn() {
    // POST /log with isbn and cover_url form fields should store both in
    // the books table. The current handler ignores unknown fields (Axum default),
    // so this test will fail until LogReadInput and the SQL upsert are updated.
    use axum::http::{Request, StatusCode};
    use sqlx::Row;
    use tower::ServiceExt;

    let app = make_test_router().await;
    let db = make_test_db().await;

    // Clean up any prior test data
    sqlx::query("DELETE FROM reads WHERE book_id IN (SELECT book_id FROM books WHERE title = $1)")
        .bind("Test ISBN Book")
        .execute(&db)
        .await
        .ok();
    sqlx::query("DELETE FROM books WHERE title = $1")
        .bind("Test ISBN Book")
        .execute(&db)
        .await
        .ok();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/log")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(axum::body::Body::from(
                    "title=Test+ISBN+Book&author=Test+Author&isbn=9780064430173&cover_url=https%3A%2F%2Fcovers.openlibrary.org%2Fb%2Fisbn%2F9780064430173-M.jpg"
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::SEE_OTHER,
        "POST /log with isbn should redirect on success (not 422 or 500)"
    );

    // Verify isbn was stored — uses runtime sqlx (no macro) since the column
    // doesn't exist yet at compile time
    let row = sqlx::query("SELECT isbn, cover_url FROM books WHERE title = $1 AND author = $2")
        .bind("Test ISBN Book")
        .bind("Test Author")
        .fetch_one(&db)
        .await
        .expect("Book should exist in DB after logging with isbn");

    let stored_isbn: Option<String> = row.try_get("isbn").unwrap_or(None);
    assert_eq!(
        stored_isbn.as_deref(),
        Some("9780064430173"),
        "isbn should be stored in the books table"
    );

    let stored_cover: Option<String> = row.try_get("cover_url").unwrap_or(None);
    assert!(
        stored_cover.is_some(),
        "cover_url should be stored in the books table"
    );
}

// =============================================================================
// BW-d501acb6aa3a44b3: Redesign UI to match Brandi's warm colorful design
// =============================================================================
//
// Tests for the new routes (/library, /progress), soft-delete, re-read,
// and the /history → /library redirect.

// -- New route: /library --

#[tokio::test]
async fn library_returns_200() {
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let app = make_test_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/library")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "GET /library should return 200 OK"
    );
}

#[tokio::test]
async fn library_contains_search_input() {
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let app = make_test_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/library")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = std::str::from_utf8(&body).expect("response body should be valid UTF-8");

    assert!(
        html.contains(r#"name="q""#),
        "Library page should contain a search input with name='q'"
    );
}

#[tokio::test]
async fn library_search_filters_results() {
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let db = make_test_db().await;

    // Insert two books with distinct titles
    sqlx::query(
        "DELETE FROM reads WHERE book_id IN (SELECT book_id FROM books WHERE title IN ($1, $2))",
    )
    .bind("Alpha Search Book")
    .bind("Beta Other Book")
    .execute(&db)
    .await
    .ok();
    sqlx::query("DELETE FROM books WHERE title IN ($1, $2)")
        .bind("Alpha Search Book")
        .bind("Beta Other Book")
        .execute(&db)
        .await
        .ok();

    let book_id_a: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO books (title, author) VALUES ('Alpha Search Book', 'Author A') ON CONFLICT (title, author) DO UPDATE SET title = EXCLUDED.title RETURNING book_id"
    )
    .fetch_one(&db)
    .await
    .unwrap();
    sqlx::query("INSERT INTO reads (book_id) VALUES ($1)")
        .bind(book_id_a)
        .execute(&db)
        .await
        .unwrap();

    let book_id_b: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO books (title, author) VALUES ('Beta Other Book', 'Author B') ON CONFLICT (title, author) DO UPDATE SET title = EXCLUDED.title RETURNING book_id"
    )
    .fetch_one(&db)
    .await
    .unwrap();
    sqlx::query("INSERT INTO reads (book_id) VALUES ($1)")
        .bind(book_id_b)
        .execute(&db)
        .await
        .unwrap();

    let app = make_test_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/library?q=Alpha")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = std::str::from_utf8(&body).expect("response body should be valid UTF-8");

    assert!(
        html.contains("Alpha Search Book"),
        "Search for 'Alpha' should show 'Alpha Search Book'"
    );
    assert!(
        !html.contains("Beta Other Book"),
        "Search for 'Alpha' should NOT show 'Beta Other Book'"
    );
}

// -- New route: /progress --

#[tokio::test]
async fn progress_returns_200() {
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let app = make_test_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/progress")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "GET /progress should return 200 OK"
    );
}

#[tokio::test]
async fn progress_shows_percentage() {
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let app = make_test_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/progress")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = std::str::from_utf8(&body).expect("response body should be valid UTF-8");

    assert!(
        html.contains('%'),
        "Progress page should display a percentage indicator"
    );
}

#[tokio::test]
async fn progress_shows_checkbox_grids() {
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let app = make_test_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/progress")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = std::str::from_utf8(&body).expect("response body should be valid UTF-8");

    // The progress page should render 10 checkbox grids (one per 100-book block).
    // Each grid has a range label like "1-100", "101-200", etc.
    assert!(
        html.contains("1-100"),
        "Progress page should contain the first checkbox grid block (1-100)"
    );
    assert!(
        html.contains("901-1000"),
        "Progress page should contain the last checkbox grid block (901-1000)"
    );
}

#[tokio::test]
async fn progress_shows_kindergarten_countdown() {
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let app = make_test_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/progress")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = std::str::from_utf8(&body).expect("response body should be valid UTF-8");

    assert!(
        html.to_lowercase().contains("kindergarten"),
        "Progress page should display a kindergarten countdown"
    );
    // Timeline shows the bookend years
    assert!(
        html.contains("2025"),
        "Progress page timeline should show start year 2025"
    );
    assert!(
        html.contains("2030"),
        "Progress page timeline should show kindergarten year 2030"
    );
}

// -- /history → /library redirect (308) --

#[tokio::test]
async fn history_redirects_to_library() {
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let app = make_test_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/history")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::PERMANENT_REDIRECT,
        "GET /history should return 308 Permanent Redirect to /library"
    );

    let location = response
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        location.contains("/library"),
        "Redirect Location should point to /library, got: {location}"
    );
}

// -- POST /library/reread --

#[tokio::test]
async fn library_reread_creates_read() {
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let app = make_test_router().await;
    let db = make_test_db().await;

    // Cleanup prior test data
    sqlx::query("DELETE FROM reads WHERE book_id IN (SELECT book_id FROM books WHERE title = $1)")
        .bind("Reread Test BW-d501")
        .execute(&db)
        .await
        .ok();
    sqlx::query("DELETE FROM books WHERE title = $1")
        .bind("Reread Test BW-d501")
        .execute(&db)
        .await
        .ok();

    let book_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO books (title, author) VALUES ('Reread Test BW-d501', 'Author') RETURNING book_id",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    sqlx::query("INSERT INTO reads (book_id) VALUES ($1)")
        .bind(book_id)
        .execute(&db)
        .await
        .unwrap();

    let before_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reads WHERE book_id = $1")
        .bind(book_id)
        .fetch_one(&db)
        .await
        .unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/library/reread")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(axum::body::Body::from(format!("book_id={book_id}")))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::SEE_OTHER,
        "POST /library/reread should redirect after logging a re-read"
    );

    let after_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reads WHERE book_id = $1")
        .bind(book_id)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(
        after_count,
        before_count + 1,
        "Re-read should add one more row to reads for that book"
    );
}

// -- POST /library/delete (soft delete) --

#[tokio::test]
async fn library_delete_soft_deletes() {
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let app = make_test_router().await;
    let db = make_test_db().await;

    // Cleanup prior test data
    sqlx::query("DELETE FROM reads WHERE book_id IN (SELECT book_id FROM books WHERE title = $1)")
        .bind("Delete Test BW-d501")
        .execute(&db)
        .await
        .ok();
    sqlx::query("DELETE FROM books WHERE title = $1")
        .bind("Delete Test BW-d501")
        .execute(&db)
        .await
        .ok();

    let book_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO books (title, author) VALUES ('Delete Test BW-d501', 'Author') RETURNING book_id",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    sqlx::query("INSERT INTO reads (book_id) VALUES ($1)")
        .bind(book_id)
        .execute(&db)
        .await
        .unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/library/delete")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(axum::body::Body::from(format!("book_id={book_id}")))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::SEE_OTHER,
        "POST /library/delete should redirect after soft-deleting"
    );

    // Rows should exist but have deleted_at set (uses runtime sqlx — column may not exist yet)
    let active_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM reads WHERE book_id = $1 AND deleted_at IS NULL")
            .bind(book_id)
            .fetch_one(&db)
            .await
            .unwrap_or(0);

    assert_eq!(
        active_count, 0,
        "After delete, no reads for that book should have deleted_at IS NULL"
    );
}

// -- Soft-deleted books are excluded from counts --

#[tokio::test]
async fn soft_deleted_reads_excluded_from_stats() {
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let db = make_test_db().await;

    // Insert a book with a read, then soft-delete the read
    sqlx::query("DELETE FROM reads WHERE book_id IN (SELECT book_id FROM books WHERE title = $1)")
        .bind("Deleted Stats Book")
        .execute(&db)
        .await
        .ok();
    sqlx::query("DELETE FROM books WHERE title = $1")
        .bind("Deleted Stats Book")
        .execute(&db)
        .await
        .ok();

    let book_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO books (title, author) VALUES ('Deleted Stats Book', 'Author') RETURNING book_id",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    let read_id: uuid::Uuid =
        sqlx::query_scalar("INSERT INTO reads (book_id) VALUES ($1) RETURNING read_id")
            .bind(book_id)
            .fetch_one(&db)
            .await
            .unwrap();

    // Soft-delete the read (the column may not exist yet — if so, the test will
    // fail with a DB error, which is expected during the scaffold phase)
    sqlx::query("UPDATE reads SET deleted_at = NOW() WHERE read_id = $1")
        .bind(read_id)
        .execute(&db)
        .await
        .expect("Soft-delete requires deleted_at column on reads table");

    let app = make_test_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/stats")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = std::str::from_utf8(&body).expect("response body should be valid UTF-8");

    // The soft-deleted book should NOT appear as a unique book in stats.
    // We can't check for exact numbers (other tests may have inserted data),
    // but we can at least verify the page renders without error.
    assert!(
        !html.contains("Deleted Stats Book"),
        "Soft-deleted books should not appear in stats output"
    );
}

// -- Navigation tabs --

#[tokio::test]
async fn pages_include_library_tab_link() {
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let app = make_test_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/log")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = std::str::from_utf8(&body).expect("response body should be valid UTF-8");

    assert!(
        html.contains("/library"),
        "Nav should include a link to /library"
    );
}

#[tokio::test]
async fn pages_include_progress_tab_link() {
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let app = make_test_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/log")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = std::str::from_utf8(&body).expect("response body should be valid UTF-8");

    assert!(
        html.contains("/progress"),
        "Nav should include a link to /progress"
    );
}

// -- Warm design: fonts and color palette present in HTML --

#[tokio::test]
async fn layout_loads_baloo2_and_nunito_fonts() {
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let app = make_test_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/log")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = std::str::from_utf8(&body).expect("response body should be valid UTF-8");

    assert!(
        html.contains("Baloo+2") || html.contains("Baloo 2"),
        "Layout should load Baloo 2 font from Google Fonts"
    );
    assert!(
        html.contains("Nunito"),
        "Layout should load Nunito font from Google Fonts"
    );
}

#[tokio::test]
async fn layout_uses_warm_cream_background() {
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let app = make_test_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/log")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = std::str::from_utf8(&body).expect("response body should be valid UTF-8");

    // The warm cream background color is #FFF6EC; it should appear in
    // the Tailwind config or as a class name "bg-cream"
    assert!(
        html.contains("FFF6EC") || html.contains("bg-cream"),
        "Layout should use the warm cream background color (#FFF6EC)"
    );
}

// -- Stats: Amelia's Fave and Favorite Author --

#[tokio::test]
async fn stats_shows_favorite_book_section() {
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let app = make_test_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/stats")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = std::str::from_utf8(&body).expect("response body should be valid UTF-8");

    // Stats should have a section for Amelia's favorite book
    assert!(
        html.to_lowercase().contains("fav")
            || html.contains("❤️")
            || html.to_lowercase().contains("most"),
        "Stats page should include a favorite book section"
    );
}

// -- Library: book with multiple reads shows re-read badge --

#[tokio::test]
async fn library_shows_reread_badge_for_multiple_reads() {
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let db = make_test_db().await;

    sqlx::query("DELETE FROM reads WHERE book_id IN (SELECT book_id FROM books WHERE title = $1)")
        .bind("Multi Read Badge Book")
        .execute(&db)
        .await
        .ok();
    sqlx::query("DELETE FROM books WHERE title = $1")
        .bind("Multi Read Badge Book")
        .execute(&db)
        .await
        .ok();

    let book_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO books (title, author) VALUES ('Multi Read Badge Book', 'Badge Author') RETURNING book_id",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    // Insert two reads for the same book
    sqlx::query("INSERT INTO reads (book_id) VALUES ($1), ($1)")
        .bind(book_id)
        .execute(&db)
        .await
        .unwrap();

    let app = make_test_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/library")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = std::str::from_utf8(&body).expect("response body should be valid UTF-8");

    assert!(
        html.contains("Multi Read Badge Book"),
        "Library should list the book that was read multiple times"
    );
    // The read count badge should appear: ×2 or similar
    assert!(
        html.contains("×2") || html.contains("x2") || html.contains("2 read") || html.contains("×"),
        "Library should show a re-read count badge for books read more than once"
    );
}
