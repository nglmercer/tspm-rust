use reqwest::Client;
use tracing::{info, warn};
use tspm_core::{TspmEvent, WebhookConfig};

/// Sends events to webhook endpoints
pub struct WebhookService {
    client: Client,
    webhooks: Vec<WebhookConfig>,
}

impl WebhookService {
    pub fn new(webhooks: Vec<WebhookConfig>) -> Self {
        Self {
            client: Client::new(),
            webhooks: webhooks.into_iter().filter(|w| w.enabled).collect(),
        }
    }

    /// Send an event to all matching webhooks
    pub async fn send(&self, event: &TspmEvent) {
        let event_type = format!("{:?}", event.event_type());

        for webhook in &self.webhooks {
            if !webhook.events.is_empty() && !webhook.events.iter().any(|e| e == &event_type) {
                continue;
            }

            let payload = serde_json::json!({
                "event": event_type,
                "data": event,
                "timestamp": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            });

            match self.client
                .post(&webhook.url)
                .json(&payload)
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    info!("[TSPM] Webhook sent to {}: {}", webhook.url, event_type);
                }
                Ok(resp) => {
                    warn!("[TSPM] Webhook failed {}: HTTP {}", webhook.url, resp.status());
                }
                Err(e) => {
                    warn!("[TSPM] Webhook error {}: {}", webhook.url, e);
                }
            }
        }
    }
}
