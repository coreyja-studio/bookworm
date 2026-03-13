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
// All tests are #[ignore] because they require:
//   1. `AppState::for_testing(db: sqlx::PgPool) -> Self` to be added (fields
//      are currently private with no public test constructor)
//   2. The `/api/isbn/{isbn}` route to be implemented
//   3. Migration adding `isbn` and `cover_url` columns to the `books` table
//   4. `ts/dist/scanner.js` to exist (compiled from `ts/scanner.ts`) so
//      `include_str!("../ts/dist/scanner.js")` compiles
//
// Implementation agent: after completing the above, replace `todo!()` in
// `make_test_router()` below with the real construction, then un-ignore.

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
#[ignore = "Requires AppState::for_testing() and /api/isbn/{isbn} route (plan steps 7-8)"]
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
#[ignore = "Requires AppState::for_testing() and /api/isbn/{isbn} route (plan steps 7-8)"]
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
#[ignore = "Requires AppState::for_testing() and scan UI added to log_form (ts/dist/scanner.js must exist, plan steps 10-11)"]
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
#[ignore = "Requires AppState::for_testing(), isbn/cover_url DB columns (plan step 1), and updated log_read handler (plan step 9)"]
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
