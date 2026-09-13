use super::*;
use serde_json::Value;
use std::io::Write;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use tracing::instrument::WithSubscriber;

#[derive(Clone, Default)]
struct Output(Arc<Mutex<Vec<u8>>>);
impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Output {
    type Writer = Self;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}
async fn capture<T>(work: impl std::future::Future<Output = T>) -> (T, Vec<Value>) {
    let output = Output::default();
    let subscriber = tracing_subscriber::fmt()
        .json()
        .with_ansi(false)
        .with_writer(output.clone())
        .finish();
    let result = work.with_subscriber(subscriber).await;
    let bytes = output.0.lock().unwrap();
    let events = std::str::from_utf8(&bytes)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    (result, events)
}
fn types(events: &[Value]) -> Vec<&str> {
    events
        .iter()
        .filter_map(|event| event["fields"]["event_type"].as_str())
        .collect()
}

#[tokio::test]
async fn successful_sends_report_acceptance_without_recipient_details() {
    let transport = lettre::transport::stub::AsyncStubTransport::new_ok();
    let from = "from@example.test".parse().unwrap();
    let recipients = parse_recipients("one@example.test, two@example.test").unwrap();
    let (result, events) = capture(attempt(send_messages(
        &transport,
        &from,
        &recipients,
        "test",
        "<p>test</p>",
    )))
    .await;
    result.unwrap();
    assert_eq!(transport.messages().await.len(), 2);
    assert_eq!(
        types(&events),
        [
            "bookworm.email_started",
            "bookworm.smtp_accepted",
            "bookworm.smtp_accepted",
            "bookworm.email_completed"
        ]
    );
    assert_eq!(events[1]["fields"]["recipient_index"], 1);
    assert_eq!(events[2]["fields"]["recipient_index"], 2);
    assert_eq!(events[3]["fields"]["recipient_count"], 2);
    for event in &events {
        assert_eq!(event["span"]["name"], "bookworm.weekly_email");
    }
    let encoded = serde_json::to_string(&events).unwrap();
    for private in [
        "one@example.test",
        "two@example.test",
        "from@example.test",
        "<p>test</p>",
    ] {
        assert!(!encoded.contains(private));
    }
}

#[derive(Default)]
struct FailAfterFirst(AtomicUsize);
#[async_trait::async_trait]
impl AsyncTransport for FailAfterFirst {
    type Ok = ();
    type Error = std::io::Error;
    async fn send_raw(&self, _: &lettre::address::Envelope, _: &[u8]) -> std::io::Result<()> {
        if self.0.fetch_add(1, Ordering::SeqCst) == 0 {
            Ok(())
        } else {
            Err(std::io::Error::other("stub SMTP rejected message"))
        }
    }
}

#[tokio::test]
async fn partial_failure_records_accepted_messages_and_does_not_claim_completion() {
    let transport = FailAfterFirst::default();
    let from = "from@example.test".parse().unwrap();
    let recipients =
        parse_recipients("one@example.test,two@example.test,three@example.test").unwrap();
    let (result, events) = capture(attempt(send_messages(
        &transport,
        &from,
        &recipients,
        "test",
        "<p>test</p>",
    )))
    .await;
    assert!(result.is_err());
    assert_eq!(
        transport.0.load(Ordering::SeqCst),
        2,
        "stop at first rejection"
    );
    assert_eq!(
        types(&events),
        [
            "bookworm.email_started",
            "bookworm.smtp_accepted",
            "bookworm.email_failed"
        ]
    );
    assert!(
        events[2]["fields"]["error"]
            .as_str()
            .unwrap()
            .contains("stub SMTP rejected message")
    );
}

#[sqlx::test]
async fn no_reads_skips_email_without_needing_smtp_configuration(pool: sqlx::PgPool) {
    let (result, events) = capture(send_weekly_email(AppState::for_testing(pool))).await;
    result.unwrap();
    assert_eq!(
        types(&events),
        ["bookworm.email_started", "bookworm.email_skipped"]
    );
    let skip = events
        .iter()
        .find(|event| event["fields"]["event_type"] == "bookworm.email_skipped")
        .unwrap();
    assert_eq!(skip["fields"]["reason"], "no_reads");
}

#[test]
fn empty_and_malformed_recipient_lists_are_errors() {
    for value in ["", " , , ", "not an email"] {
        assert!(parse_recipients(value).is_err());
    }
    assert_eq!(parse_recipients(" , one@example.test, ").unwrap().len(), 1);
}
