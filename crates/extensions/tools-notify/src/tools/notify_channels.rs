//! Network-based notification channel implementations (Slack, Telegram, Webhook).

use super::notify_types::NotifySendResponse;
use super::notify_send::NotifySendTool;

impl NotifySendTool {
    /// Send notification via Slack webhook.
    pub(crate) async fn send_slack(
        &self,
        title: Option<&str>,
        message: &str,
        options: &serde_json::Value,
    ) -> NotifySendResponse {
        let env_webhook = std::env::var("SLACK_WEBHOOK_URL").ok();
        let webhook_url = options
            .get("webhook_url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or(env_webhook);

        let Some(url) = webhook_url else {
            return NotifySendResponse {
                success: false,
                channel: "slack".to_string(),
                message: "Slack webhook URL not configured. Set SLACK_WEBHOOK_URL or provide webhook_url in options.".to_string(),
                delivery_id: None,
            };
        };

        let text = match title {
            Some(t) => format!("*{}*\n{}", t, message),
            None => message.to_string(),
        };

        let payload = serde_json::json!({
            "text": text
        });

        match reqwest::Client::new()
            .post(url)
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => NotifySendResponse {
                success: true,
                channel: "slack".to_string(),
                message: "Notification sent to Slack".to_string(),
                delivery_id: None,
            },
            Ok(resp) => NotifySendResponse {
                success: false,
                channel: "slack".to_string(),
                message: format!("Slack API error: {}", resp.status()),
                delivery_id: None,
            },
            Err(e) => NotifySendResponse {
                success: false,
                channel: "slack".to_string(),
                message: format!("Failed to send Slack notification: {}", e),
                delivery_id: None,
            },
        }
    }

    /// Send notification via Telegram.
    pub(crate) async fn send_telegram(
        &self,
        title: Option<&str>,
        message: &str,
        options: &serde_json::Value,
    ) -> NotifySendResponse {
        let bot_token = std::env::var("TELEGRAM_BOT_TOKEN").ok();
        let env_chat_id = std::env::var("TELEGRAM_CHAT_ID").ok();
        let chat_id = options
            .get("to")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or(env_chat_id);

        let (Some(token), Some(chat)) = (bot_token, chat_id) else {
            return NotifySendResponse {
                success: false,
                channel: "telegram".to_string(),
                message: "Telegram not configured. Set TELEGRAM_BOT_TOKEN and TELEGRAM_CHAT_ID.".to_string(),
                delivery_id: None,
            };
        };

        let text = match title {
            Some(t) => format!("*{}*\n{}", t, message),
            None => message.to_string(),
        };

        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            token
        );

        let payload = serde_json::json!({
            "chat_id": chat,
            "text": text,
            "parse_mode": "Markdown"
        });

        match reqwest::Client::new()
            .post(&url)
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let body: serde_json::Value = resp.json().await.unwrap_or_default();
                let message_id = body
                    .get("result")
                    .and_then(|r| r.get("message_id"))
                    .and_then(|m| m.as_i64())
                    .map(|m| m.to_string());

                NotifySendResponse {
                    success: true,
                    channel: "telegram".to_string(),
                    message: "Notification sent to Telegram".to_string(),
                    delivery_id: message_id,
                }
            }
            Ok(resp) => NotifySendResponse {
                success: false,
                channel: "telegram".to_string(),
                message: format!("Telegram API error: {}", resp.status()),
                delivery_id: None,
            },
            Err(e) => NotifySendResponse {
                success: false,
                channel: "telegram".to_string(),
                message: format!("Failed to send Telegram notification: {}", e),
                delivery_id: None,
            },
        }
    }

    /// Send notification via generic webhook.
    pub(crate) async fn send_webhook(
        &self,
        title: Option<&str>,
        message: &str,
        priority: &str,
        options: &serde_json::Value,
    ) -> NotifySendResponse {
        let webhook_url = options.get("webhook_url").and_then(|v| v.as_str());

        let Some(url) = webhook_url else {
            return NotifySendResponse {
                success: false,
                channel: "webhook".to_string(),
                message: "webhook_url is required in options".to_string(),
                delivery_id: None,
            };
        };

        let payload = serde_json::json!({
            "title": title,
            "message": message,
            "priority": priority,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        match reqwest::Client::new()
            .post(url)
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => NotifySendResponse {
                success: true,
                channel: "webhook".to_string(),
                message: format!("Notification sent to webhook: {}", url),
                delivery_id: None,
            },
            Ok(resp) => NotifySendResponse {
                success: false,
                channel: "webhook".to_string(),
                message: format!("Webhook error: {}", resp.status()),
                delivery_id: None,
            },
            Err(e) => NotifySendResponse {
                success: false,
                channel: "webhook".to_string(),
                message: format!("Failed to send webhook notification: {}", e),
                delivery_id: None,
            },
        }
    }
}
