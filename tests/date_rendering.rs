use axum::{body::Body, http::Request};
use bookworm::{AppState, routes};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;

async fn page(pool: &PgPool, path: &str) -> String {
    let response = routes(AppState::for_testing(pool.clone()))
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    String::from_utf8(response.into_body().collect().await.unwrap().to_bytes().to_vec()).unwrap()
}

async fn book(pool: &PgPool, title: &str) -> uuid::Uuid {
    sqlx::query_scalar("INSERT INTO books (title) VALUES ($1) RETURNING book_id")
        .bind(title)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn read(pool: &PgPool, book_id: uuid::Uuid, date: &str) {
    sqlx::query("INSERT INTO reads (book_id, read_date) VALUES ($1, $2::date)")
        .bind(book_id)
        .bind(date)
        .execute(pool)
        .await
        .unwrap();
}

#[sqlx::test]
async fn history_renders_fixed_dates_newest_first(pool: PgPool) {
    let older = book(&pool, "January Book").await;
    let newer = book(&pool, "March Book").await;
    read(&pool, older, "2026-01-05").await;
    read(&pool, newer, "2026-03-14").await;
    let html = page(&pool, "/history").await;
    let march = html.find("Saturday, March 14, 2026").unwrap();
    let january = html.find("Monday, January 5, 2026").unwrap();
    assert!(march < january);
}

#[sqlx::test]
async fn book_detail_renders_first_last_and_timeline_dates(pool: PgPool) {
    let id = book(&pool, "Twice Read").await;
    read(&pool, id, "2026-01-05").await;
    read(&pool, id, "2026-03-14").await;
    let html = page(&pool, &format!("/books/{id}")).await;
    assert!(html.contains("Jan 5, 2026</div><div class=\"text-xs text-subtext\">First Read"));
    assert!(html.contains("Mar 14, 2026</div><div class=\"text-xs text-subtext\">Last Read"));
    assert!(html.contains("January 5, 2026"));
    assert!(html.contains("March 14, 2026"));
}

#[sqlx::test]
async fn library_uses_latest_read_date_for_value_and_recent_sort(pool: PgPool) {
    let older = book(&pool, "A Older").await;
    let newer = book(&pool, "Z Newer").await;
    read(&pool, older, "2026-01-05").await;
    read(&pool, newer, "2026-02-01").await;
    read(&pool, newer, "2026-03-14").await;
    let html = page(&pool, "/library?sort=recent").await;
    assert!(html.contains("Last read: 2026-03-14"));
    assert!(html.contains("Last read: 2026-01-05"));
    assert!(html.find("Z Newer").unwrap() < html.find("A Older").unwrap());
}

#[sqlx::test]
async fn progress_uses_utc_calendar_countdown(pool: PgPool) {
    async fn days_left(pool: &PgPool) -> i32 {
        sqlx::query_scalar("SELECT GREATEST(DATE '2030-09-01' - (now() AT TIME ZONE 'UTC')::date, 0)")
            .fetch_one(pool)
            .await
            .unwrap()
    }
    let before = days_left(&pool).await;
    let html = page(&pool, "/progress").await;
    let after = days_left(&pool).await;
    let prefix = &html[..html.find("Days Left").unwrap()];
    assert!([before, after].iter().any(|days| prefix.ends_with(&format!("{days}</div><div class=\"text-xs text-subtext\">"))));
}
