//! Tests for weekly reading wrap-up email (BW-2515e7a36a3044bd).

use bookworm::WeeklyStats;

// ---------------------------------------------------------------------------
// gather_weekly_stats — database tests
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn gather_weekly_stats_counts_this_week_only(pool: sqlx::PgPool) {
    // Insert 2 books
    sqlx::query!(
        "INSERT INTO books (book_id, title) VALUES ($1, $2), ($3, $4)",
        uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        "Goodnight Moon",
        uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
        "Brown Bear",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Book A: 2 reads this week
    sqlx::query!(
        "INSERT INTO reads (book_id, read_date) VALUES ($1, CURRENT_DATE), ($1, CURRENT_DATE - INTERVAL '1 day')",
        uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
    )
    .execute(&pool)
    .await
    .unwrap();

    // Book B: 1 read this week, 1 read 10 days ago
    sqlx::query!(
        "INSERT INTO reads (book_id, read_date) VALUES ($1, CURRENT_DATE), ($1, CURRENT_DATE - INTERVAL '10 days')",
        uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
    )
    .execute(&pool)
    .await
    .unwrap();

    let stats = bookworm::gather_weekly_stats(&pool).await.unwrap();

    assert_eq!(stats.total_reads_this_week, 3);
    assert_eq!(stats.total_reads_all_time, 4);
    assert_eq!(stats.unique_books_all_time, 2);
    assert_eq!(
        stats.most_read_book.as_ref().map(|(t, _)| t.as_str()),
        Some("Goodnight Moon")
    );
    assert_eq!(stats.most_read_book.as_ref().map(|(_, c)| *c), Some(2));
    assert!(stats.busiest_day.is_some());
    assert!(stats.days_with_reads >= 1);
}

#[sqlx::test]
async fn gather_weekly_stats_new_unique_books_this_week(pool: sqlx::PgPool) {
    let old_book = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let new_book = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();

    sqlx::query!(
        "INSERT INTO books (book_id, title) VALUES ($1, $2), ($3, $4)",
        old_book,
        "Old Favorite",
        new_book,
        "Brand New Book",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Old Favorite: first read 10 days ago, re-read this week
    sqlx::query!(
        "INSERT INTO reads (book_id, read_date) VALUES ($1, CURRENT_DATE - INTERVAL '10 days'), ($1, CURRENT_DATE)",
        old_book,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Brand New Book: first read this week
    sqlx::query!(
        "INSERT INTO reads (book_id, read_date) VALUES ($1, CURRENT_DATE)",
        new_book,
    )
    .execute(&pool)
    .await
    .unwrap();

    let stats = bookworm::gather_weekly_stats(&pool).await.unwrap();

    assert_eq!(stats.new_unique_books_this_week, 1);
    assert_eq!(stats.unique_books_all_time, 2);
}

#[sqlx::test]
async fn gather_weekly_stats_empty_database(pool: sqlx::PgPool) {
    let stats = bookworm::gather_weekly_stats(&pool).await.unwrap();

    assert_eq!(stats.total_reads_this_week, 0);
    assert_eq!(stats.total_reads_all_time, 0);
    assert_eq!(stats.unique_books_all_time, 0);
    assert_eq!(stats.new_unique_books_this_week, 0);
    assert!(stats.most_read_book.is_none());
    assert!(stats.busiest_day.is_none());
    assert_eq!(stats.days_with_reads, 0);
}

#[sqlx::test]
async fn gather_weekly_stats_ignores_deleted_reads(pool: sqlx::PgPool) {
    let book_id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();

    sqlx::query!(
        "INSERT INTO books (book_id, title) VALUES ($1, $2)",
        book_id,
        "Deleted Book",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query!(
        "INSERT INTO reads (book_id, read_date, deleted_at) VALUES ($1, CURRENT_DATE, NOW())",
        book_id,
    )
    .execute(&pool)
    .await
    .unwrap();

    let stats = bookworm::gather_weekly_stats(&pool).await.unwrap();

    assert_eq!(stats.total_reads_this_week, 0);
    assert_eq!(stats.total_reads_all_time, 0);
}

#[sqlx::test]
async fn gather_weekly_stats_busiest_day_picks_highest(pool: sqlx::PgPool) {
    let book_id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();

    sqlx::query!(
        "INSERT INTO books (book_id, title) VALUES ($1, $2)",
        book_id,
        "Popular Book",
    )
    .execute(&pool)
    .await
    .unwrap();

    // 1 read yesterday
    sqlx::query!(
        "INSERT INTO reads (book_id, read_date) VALUES ($1, CURRENT_DATE - INTERVAL '1 day')",
        book_id,
    )
    .execute(&pool)
    .await
    .unwrap();

    // 3 reads today
    sqlx::query!(
        "INSERT INTO reads (book_id, read_date) VALUES ($1, CURRENT_DATE), ($1, CURRENT_DATE), ($1, CURRENT_DATE)",
        book_id,
    )
    .execute(&pool)
    .await
    .unwrap();

    let stats = bookworm::gather_weekly_stats(&pool).await.unwrap();

    assert_eq!(stats.busiest_day.as_ref().map(|(_, c)| *c), Some(3));
    assert_eq!(stats.days_with_reads, 2);
}

// ---------------------------------------------------------------------------
// build_weekly_email_html — pure function tests
// ---------------------------------------------------------------------------

#[test]
fn build_weekly_email_html_contains_stats() {
    let stats = WeeklyStats {
        total_reads_this_week: 15,
        total_reads_all_time: 200,
        unique_books_all_time: 80,
        new_unique_books_this_week: 5,
        most_read_book: Some(("Goodnight Moon".to_string(), 7)),
        busiest_day: Some(("Wednesday".to_string(), 6)),
        days_with_reads: 5,
    };

    let html = bookworm::build_weekly_email_html(&stats).into_string();

    assert!(html.contains("15"), "should contain total reads");
    assert!(html.contains("Goodnight Moon"), "should contain book title");
    assert!(html.contains("7 time"), "should contain most-read count");
    assert!(html.contains("Wednesday"), "should contain busiest day");
    assert!(html.contains("80"), "should contain unique books count");
    assert!(
        html.contains("1,000") || html.contains("1000"),
        "should contain the goal"
    );
    assert!(html.contains("5 of 7"), "should contain days with reads");
}

#[test]
fn build_weekly_email_html_handles_no_most_read_book() {
    let stats = WeeklyStats {
        total_reads_this_week: 1,
        total_reads_all_time: 1,
        unique_books_all_time: 1,
        new_unique_books_this_week: 1,
        most_read_book: None,
        busiest_day: None,
        days_with_reads: 1,
    };

    let html = bookworm::build_weekly_email_html(&stats).into_string();

    assert!(!html.is_empty(), "HTML should not be empty");
    assert!(html.contains("1 of 7"), "should contain the read count");
}

// ---------------------------------------------------------------------------
// Milestone detection
// ---------------------------------------------------------------------------

#[test]
fn build_weekly_email_html_shows_milestone_when_crossed() {
    let stats = WeeklyStats {
        total_reads_this_week: 10,
        total_reads_all_time: 300,
        unique_books_all_time: 100,
        new_unique_books_this_week: 3,
        most_read_book: None,
        busiest_day: None,
        days_with_reads: 3,
    };

    let html = bookworm::build_weekly_email_html(&stats).into_string();

    assert!(
        html.contains("Milestone") || html.contains("milestone") || html.contains("🌟"),
        "should contain milestone celebration"
    );
    assert!(
        html.contains("100"),
        "should reference the 100 milestone number"
    );
}

#[test]
fn build_weekly_email_html_no_milestone_when_not_crossed() {
    let stats = WeeklyStats {
        total_reads_this_week: 10,
        total_reads_all_time: 300,
        unique_books_all_time: 99,
        new_unique_books_this_week: 3,
        most_read_book: None,
        busiest_day: None,
        days_with_reads: 3,
    };

    let html = bookworm::build_weekly_email_html(&stats).into_string();

    assert!(
        !html.contains("🌟"),
        "should not contain milestone star when not crossed"
    );
}

#[test]
fn build_weekly_email_html_no_milestone_when_already_past() {
    let stats = WeeklyStats {
        total_reads_this_week: 10,
        total_reads_all_time: 300,
        unique_books_all_time: 105,
        new_unique_books_this_week: 2,
        most_read_book: None,
        busiest_day: None,
        days_with_reads: 3,
    };

    let html = bookworm::build_weekly_email_html(&stats).into_string();

    // 105 >= 100 && 105 - 2 = 103 >= 100 → NOT crossed this week
    assert!(
        !html.contains("🌟"),
        "should not re-celebrate 100 milestone"
    );
}

// ---------------------------------------------------------------------------
// send_weekly_email — skip on zero reads
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn send_weekly_email_skips_when_no_reads(pool: sqlx::PgPool) {
    let app_state = bookworm::AppState::for_testing(pool);
    let result = bookworm::send_weekly_email(app_state).await;
    assert!(
        result.is_ok(),
        "should return Ok when no reads (early return)"
    );
}

// ---------------------------------------------------------------------------
// Cron registration
// ---------------------------------------------------------------------------

#[test]
fn cron_registry_registers_weekly_email() {
    // Just verify it doesn't panic (cron expression parses successfully)
    let _registry = bookworm::cron_registry();
}

// ---------------------------------------------------------------------------
// Days-with-reads streak indicator
// ---------------------------------------------------------------------------

#[test]
fn build_weekly_email_html_shows_reading_streak() {
    let stats = WeeklyStats {
        total_reads_this_week: 14,
        total_reads_all_time: 100,
        unique_books_all_time: 50,
        new_unique_books_this_week: 2,
        most_read_book: Some(("Test Book".to_string(), 3)),
        busiest_day: Some(("Monday".to_string(), 4)),
        days_with_reads: 7,
    };

    let html = bookworm::build_weekly_email_html(&stats).into_string();

    assert!(html.contains("7 of 7"), "should show 7 of 7 days");
    assert!(
        html.contains("7 of 7") || html.contains("Perfect week"),
        "should indicate full-week reading"
    );
}
