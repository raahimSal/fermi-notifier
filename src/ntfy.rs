use crate::config::Config;
use crate::error::{AppError, AppResult};
use reqwest::Client;
use tracing::instrument;

#[instrument(skip(client, config, message_body), fields(topic = %config.ntfy_topic, message_len = message_body.len(), delay = ?delay))]
pub async fn send_notification(
    client: &Client,
    config: &Config,
    title_prefix: &str,
    message_body: &str,
    delay: Option<&str>,
) -> AppResult<()> {
    let url = format!("https://ntfy.sh/{}", config.ntfy_topic);

    let first_line = message_body
        .lines()
        .next()
        .unwrap_or("Fermi Notification")
        .trim();

    let title = format!("{}{}", title_prefix, first_line);
    let truncated_title: String = title.chars().take(100).collect();

    tracing::info!(url = %url, title = %truncated_title, ?delay, "Sending notification to ntfy.sh");

    let mut request_builder = client
        .post(&url)
        .header("Title", truncated_title.as_str())
        .header("Tags", "brain,puzzle")
        .body(message_body.to_string());

    // Add delay header if provided
    if let Some(d) = delay {
        if !d.is_empty() {
            // Validate delay format slightly (basic check)
            if d.ends_with('s') || d.ends_with('m') || d.ends_with('h') || d.ends_with('d') {
                request_builder = request_builder.header("X-Delay", d);
                tracing::info!("Scheduling notification with delay: {}", d);
            } else {
                tracing::warn!(
                    "Invalid delay format provided: '{}'. Sending immediately.",
                    d
                );
            }
        }
    }

    let response = request_builder.send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error body".to_string());
        tracing::error!(status = %status, error_body = %error_text, "ntfy.sh request failed");
        return Err(AppError::Ntfy(format!(
            "Notification request failed with status {}: {}",
            status, error_text
        )));
    }

    tracing::info!("Successfully sent/scheduled notification via ntfy.sh");
    Ok(())
}
