//! Test scaffold for weekly reading wrap-up email (BW-2515e7a36a3044bd).
//!
//! These tests define the expected behavior for:
//! - `gather_weekly_stats`: collects reading stats from the database
//! - `build_weekly_email_html`: renders stats into an HTML email
//! - Milestone detection logic
//! - `send_weekly_email`: skips sending when no reads occurred
//! - Cron registration
//!
//! All tests are `#[ignore]` because they depend on types/functions
//! that don't exist yet (`WeeklyStats`, `gather_weekly_stats`,
//! `build_weekly_email_html`, `cron_registry`, `send_weekly_email`).
//!
//! The implementation agent should un-ignore these and fill in the
//! function calls once the types are created.

// ---------------------------------------------------------------------------
// gather_weekly_stats — database tests
// ---------------------------------------------------------------------------

#[test]
#[ignore = "Requires gather_weekly_stats and WeeklyStats to be implemented"]
fn gather_weekly_stats_counts_this_week_only() {
    // This should be a #[sqlx::test] with pool: sqlx::PgPool parameter.
    //
    // Setup: insert 2 books, 3 reads this week and 1 read from 10 days ago.
    //
    // Book A: "Goodnight Moon" — read twice this week (CURRENT_DATE, CURRENT_DATE - 1 day)
    // Book B: "Brown Bear" — read once this week (CURRENT_DATE), once 10 days ago
    //
    // Call: bookworm::gather_weekly_stats(&pool).await.unwrap()
    //
    // Assertions:
    //   stats.total_reads_this_week == 3  (only reads within 7 days)
    //   stats.total_reads_all_time == 4   (all non-deleted reads)
    //   stats.unique_books_all_time == 2  (two distinct books)
    //   stats.most_read_book == Some(("Goodnight Moon", 2))
    //   stats.busiest_day.is_some()
    //   stats.days_with_reads >= 1
    todo!(
        "Convert to #[sqlx::test], insert test data, call gather_weekly_stats, \
         assert weekly vs all-time counts are correct"
    )
}

#[test]
#[ignore = "Requires gather_weekly_stats and WeeklyStats to be implemented"]
fn gather_weekly_stats_new_unique_books_this_week() {
    // This should be a #[sqlx::test] with pool: sqlx::PgPool parameter.
    //
    // Book A "Old Favorite": first read 10 days ago, re-read this week → NOT a new unique
    // Book B "Brand New Book": first read this week → IS a new unique
    //
    // Call: bookworm::gather_weekly_stats(&pool).await.unwrap()
    //
    // Assertions:
    //   stats.new_unique_books_this_week == 1 (only Brand New Book)
    //   stats.unique_books_all_time == 2
    todo!(
        "Convert to #[sqlx::test], insert books with reads spanning the 7-day boundary, \
         assert new_unique_books_this_week correctly excludes re-reads of old books"
    )
}

#[test]
#[ignore = "Requires gather_weekly_stats and WeeklyStats to be implemented"]
fn gather_weekly_stats_empty_database() {
    // This should be a #[sqlx::test] with pool: sqlx::PgPool parameter.
    //
    // No books, no reads — should return zeros without error.
    //
    // Call: bookworm::gather_weekly_stats(&pool).await.unwrap()
    //
    // Assertions:
    //   stats.total_reads_this_week == 0
    //   stats.total_reads_all_time == 0
    //   stats.unique_books_all_time == 0
    //   stats.new_unique_books_this_week == 0
    //   stats.most_read_book.is_none()
    //   stats.busiest_day.is_none()
    //   stats.days_with_reads == 0
    todo!(
        "Convert to #[sqlx::test], call gather_weekly_stats on empty DB, \
         assert all fields are zero/None"
    )
}

#[test]
#[ignore = "Requires gather_weekly_stats and WeeklyStats to be implemented"]
fn gather_weekly_stats_ignores_deleted_reads() {
    // This should be a #[sqlx::test] with pool: sqlx::PgPool parameter.
    //
    // Insert a book and a read with deleted_at = NOW().
    // Soft-deleted reads should not be counted in any stat.
    //
    // Call: bookworm::gather_weekly_stats(&pool).await.unwrap()
    //
    // Assertions:
    //   stats.total_reads_this_week == 0
    //   stats.total_reads_all_time == 0
    todo!(
        "Convert to #[sqlx::test], insert a soft-deleted read, \
         assert it is excluded from all counts"
    )
}

#[test]
#[ignore = "Requires gather_weekly_stats and WeeklyStats to be implemented"]
fn gather_weekly_stats_busiest_day_picks_highest() {
    // This should be a #[sqlx::test] with pool: sqlx::PgPool parameter.
    //
    // Insert 1 read yesterday and 3 reads today for the same book.
    //
    // Call: bookworm::gather_weekly_stats(&pool).await.unwrap()
    //
    // Assertions:
    //   stats.busiest_day count == 3  (today had the most reads)
    //   stats.days_with_reads == 2    (reads on 2 distinct days)
    todo!(
        "Convert to #[sqlx::test], insert reads on 2 days with different counts, \
         assert busiest_day picks the day with more reads"
    )
}

// ---------------------------------------------------------------------------
// build_weekly_email_html — pure function tests
// ---------------------------------------------------------------------------

#[test]
#[ignore = "Requires WeeklyStats and build_weekly_email_html to be implemented"]
fn build_weekly_email_html_contains_stats() {
    // Construct a WeeklyStats with known values:
    //   total_reads_this_week: 15
    //   total_reads_all_time: 200
    //   unique_books_all_time: 80
    //   new_unique_books_this_week: 5
    //   most_read_book: Some(("Goodnight Moon", 7))
    //   busiest_day: Some(("Wednesday", 6))
    //   days_with_reads: 5
    //
    // Call: bookworm::build_weekly_email_html(&stats).into_string()
    //
    // Assertions on the HTML string:
    //   contains "15" (total reads this week)
    //   contains "Goodnight Moon" (most-read book title)
    //   contains "7" (most-read book count)
    //   contains "Wednesday" (busiest day name)
    //   contains "80" (unique books count)
    //   contains "1,000" or "1000" (the goal)
    //   contains "5" (new unique books or days with reads)
    todo!(
        "Construct WeeklyStats, call build_weekly_email_html, \
         assert HTML contains expected stat values"
    )
}

#[test]
#[ignore = "Requires WeeklyStats and build_weekly_email_html to be implemented"]
fn build_weekly_email_html_handles_no_most_read_book() {
    // Construct a WeeklyStats where most_read_book and busiest_day are None.
    //   total_reads_this_week: 1
    //   most_read_book: None
    //   busiest_day: None
    //   days_with_reads: 1
    //
    // Call: bookworm::build_weekly_email_html(&stats).into_string()
    //
    // Assertions:
    //   HTML is non-empty (no panic)
    //   contains "1" (the read count)
    todo!(
        "Construct WeeklyStats with None fields, call build_weekly_email_html, \
         assert it produces valid HTML without panicking"
    )
}

// ---------------------------------------------------------------------------
// Milestone detection
// ---------------------------------------------------------------------------

#[test]
#[ignore = "Requires WeeklyStats and build_weekly_email_html to be implemented"]
fn build_weekly_email_html_shows_milestone_when_crossed() {
    // Milestone detection formula:
    //   unique_books_all_time >= milestone && unique_books_all_time - new_unique_books_this_week < milestone
    //
    // Test case: unique_books_all_time=100, new_unique_books_this_week=3
    //   → was at 97 before this week → crossed the 100 milestone
    //
    // Call: bookworm::build_weekly_email_html(&stats).into_string()
    //
    // Assertions:
    //   HTML contains "100" AND some milestone indicator ("milestone", "🌟", or "Milestone")
    todo!(
        "Construct WeeklyStats that crosses the 100 milestone this week, \
         assert HTML contains milestone celebration"
    )
}

#[test]
#[ignore = "Requires WeeklyStats and build_weekly_email_html to be implemented"]
fn build_weekly_email_html_no_milestone_when_not_crossed() {
    // Test case: unique_books_all_time=99, new_unique_books_this_week=3
    //   → hasn't reached 100 yet → no milestone
    //
    // Call: bookworm::build_weekly_email_html(&stats).into_string()
    //
    // Assertions:
    //   HTML does NOT contain "milestone" or "🌟"
    todo!(
        "Construct WeeklyStats just below 100 milestone, \
         assert HTML does not contain any milestone celebration"
    )
}

#[test]
#[ignore = "Requires WeeklyStats and build_weekly_email_html to be implemented"]
fn build_weekly_email_html_no_milestone_when_already_past() {
    // Test case: unique_books_all_time=105, new_unique_books_this_week=2
    //   → was at 103 before, already past 100 → should NOT re-celebrate
    //   Formula: 105 >= 100 && 105 - 2 = 103 >= 100 → NOT crossed this week
    //
    // Call: bookworm::build_weekly_email_html(&stats).into_string()
    //
    // Assertions:
    //   HTML does NOT contain "🌟" (100-milestone star)
    todo!(
        "Construct WeeklyStats where 100 milestone was already crossed in a prior week, \
         assert HTML does not re-celebrate it"
    )
}

// ---------------------------------------------------------------------------
// send_weekly_email — skip on zero reads
// ---------------------------------------------------------------------------

#[test]
#[ignore = "Requires send_weekly_email and gather_weekly_stats to be implemented"]
fn send_weekly_email_skips_when_no_reads() {
    // This should be a #[sqlx::test] with pool: sqlx::PgPool parameter.
    //
    // With an empty database (no reads), send_weekly_email should return Ok
    // without attempting to call the Resend API.
    //
    // Setup: let app_state = bookworm::AppState::for_testing(pool);
    // Call:  bookworm::send_weekly_email(app_state).await
    //
    // This should succeed even without RESEND_API_KEY being set,
    // since it should bail before reaching the API call.
    //
    // Assertions:
    //   result.is_ok()
    todo!(
        "Convert to #[sqlx::test], call send_weekly_email on empty DB, \
         assert it returns Ok (early return, no email sent)"
    )
}

// ---------------------------------------------------------------------------
// Cron registration
// ---------------------------------------------------------------------------

#[test]
#[ignore = "Requires cron_registry to be implemented"]
fn cron_registry_registers_weekly_email() {
    // Call: bookworm::cron_registry()
    //
    // This verifies the cron expression parses correctly and the
    // job is registered without panicking. The cron expression should
    // be "0 0 18 * * Sun *" (every Sunday at 6 PM).
    //
    // Assertions:
    //   Function returns without panic (implicit)
    todo!(
        "Call bookworm::cron_registry(), verify it returns a valid registry \
         without panicking (cron expression parses successfully)"
    )
}

// ---------------------------------------------------------------------------
// Days-with-reads streak indicator
// ---------------------------------------------------------------------------

#[test]
#[ignore = "Requires WeeklyStats and build_weekly_email_html to be implemented"]
fn build_weekly_email_html_shows_reading_streak() {
    // Construct a WeeklyStats with days_with_reads=7 (full week).
    //
    // Call: bookworm::build_weekly_email_html(&stats).into_string()
    //
    // Assertions:
    //   HTML contains "7" and "7" (indicating 7 of 7 days)
    todo!(
        "Construct WeeklyStats with 7/7 reading days, \
         assert HTML shows full-week reading streak indicator"
    )
}
