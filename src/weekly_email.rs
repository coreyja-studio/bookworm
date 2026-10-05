use crate::{AppState, Result, build_weekly_email_html, gather_weekly_stats};
use color_eyre::eyre::{Context as _, eyre};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, header::ContentType},
    transport::smtp::authentication::Credentials,
};

#[cfg(test)]
mod tests;

pub async fn send_weekly_email(app_state: AppState) -> Result<()> {
    attempt(send(app_state)).await
}

#[tracing::instrument(name = "bookworm.weekly_email", skip_all)]
async fn attempt(work: impl std::future::Future<Output = Result<()>>) -> Result<()> {
    tracing::info!(
        event_type = "bookworm.email_started",
        "Weekly email attempt started"
    );
    let result = work.await;
    if let Err(error) = &result {
        tracing::error!(event_type = "bookworm.email_failed", error = %format!("{error:#}"),
            "Weekly email attempt failed");
    }
    result
}

async fn send(app_state: AppState) -> Result<()> {
    let stats = gather_weekly_stats(&app_state.db).await?;
    if stats.total_reads_this_week == 0 {
        tracing::info!(
            event_type = "bookworm.email_skipped",
            reason = "no_reads",
            "No reads this week, skipping weekly email"
        );
        return Ok(());
    }
    let html = build_weekly_email_html(&stats).into_string();
    let smtp_host = std::env::var("SMTP_HOST").wrap_err("SMTP_HOST must be set")?;
    let smtp_username = std::env::var("SMTP_USERNAME").wrap_err("SMTP_USERNAME must be set")?;
    let smtp_password = std::env::var("SMTP_PASSWORD").wrap_err("SMTP_PASSWORD must be set")?;
    let from_address = std::env::var("SMTP_FROM")
        .unwrap_or_else(|_| "Bookworm <bookworm@updates.coreyja.com>".to_string());
    let recipients =
        std::env::var("WEEKLY_EMAIL_RECIPIENTS").wrap_err("WEEKLY_EMAIL_RECIPIENTS must be set")?;
    let from: Mailbox = from_address.parse().wrap_err("Invalid SMTP_FROM address")?;
    let recipients = parse_recipients(&recipients)?;
    let subject = format!(
        "\u{1f4da} Amelia's Week: {} reads!",
        stats.total_reads_this_week
    );
    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&smtp_host)
            .wrap_err("Failed to create SMTP transport")?
            .credentials(Credentials::new(smtp_username, smtp_password))
            .build();
    send_messages(&mailer, &from, &recipients, &subject, &html).await
}

fn parse_recipients(value: &str) -> Result<Vec<Mailbox>> {
    let recipients: Vec<Mailbox> = value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::parse)
        .collect::<std::result::Result<_, _>>()
        .wrap_err("Invalid recipient address in WEEKLY_EMAIL_RECIPIENTS")?;
    if recipients.is_empty() {
        return Err(eyre!(
            "WEEKLY_EMAIL_RECIPIENTS must contain at least one address"
        ));
    }
    Ok(recipients)
}

async fn send_messages<T: AsyncTransport + Sync>(
    mailer: &T,
    from: &Mailbox,
    recipients: &[Mailbox],
    subject: &str,
    html: &str,
) -> Result<()>
where
    T::Error: std::error::Error + Send + Sync + 'static,
{
    for (index, to) in recipients.iter().enumerate() {
        let email = Message::builder()
            .from(from.clone())
            .to(to.clone())
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(html.to_owned())
            .wrap_err("Failed to build email message")?;
        mailer
            .send(email)
            .await
            .wrap_err("Failed to send weekly email via SMTP")?;
        tracing::info!(
            event_type = "bookworm.smtp_accepted",
            recipient_index = index + 1,
            recipient_count = recipients.len(),
            "SMTP accepted weekly email"
        );
    }
    tracing::info!(
        event_type = "bookworm.email_completed",
        recipient_count = recipients.len(),
        "Weekly email accepted for all recipients"
    );
    Ok(())
}
