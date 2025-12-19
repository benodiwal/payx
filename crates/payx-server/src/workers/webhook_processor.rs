use std::time::Duration;

use chrono::Utc;
use reqwest::Client;
use sqlx::PgPool;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::domain::{sign_payload, Business, WebhookOutbox};

pub struct WebhookProcessor {
    pool: PgPool,
    client: Client,
    handle: Option<JoinHandle<()>>,
}

impl WebhookProcessor {
    pub fn new(pool: PgPool, client: Client) -> Self {
        Self {
            pool,
            client,
            handle: None,
        }
    }

    pub fn start(&mut self) {
        let pool = self.pool.clone();
        let client = self.client.clone();

        let handle = tokio::spawn(async move {
            loop {
                if let Err(e) = process_batch(&pool, &client).await {
                    error!(error = %e, "webhook processing error");
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        self.handle = Some(handle);
    }
}

async fn process_batch(pool: &PgPool, client: &Client) -> anyhow::Result<()> {
    let events: Vec<WebhookOutbox> = sqlx::query_as(
        r#"
        SELECT * FROM webhook_outbox
        WHERE status IN ('pending', 'retrying')
        AND next_attempt_at <= NOW()
        ORDER BY created_at
        LIMIT 100
        FOR UPDATE SKIP LOCKED
        "#,
    )
    .fetch_all(pool)
    .await?;

    for event in events {
        match deliver_webhook(pool, client, &event).await {
            Ok(_) => mark_delivered(pool, event.id).await?,
            Err(e) => schedule_retry(pool, event.id, &e.to_string()).await?,
        }
    }

    Ok(())
}

async fn deliver_webhook(
    pool: &PgPool,
    client: &Client,
    event: &WebhookOutbox,
) -> anyhow::Result<()> {
    let business: Option<Business> = sqlx::query_as("SELECT * FROM businesses WHERE id = $1")
        .bind(event.business_id)
        .fetch_optional(pool)
        .await?;

    let business = match business {
        Some(b) => b,
        None => {
            warn!(business_id = %event.business_id, "business not found, marking webhook as failed");
            return Err(anyhow::anyhow!("business not found"));
        }
    };

    let webhook_url = match &business.webhook_url {
        Some(url) => url,
        None => {
            info!(business_id = %event.business_id, "no webhook url configured");
            return Ok(());
        }
    };

    let payload = serde_json::to_vec(&event.payload)?;
    let signature = business
        .webhook_secret
        .as_ref()
        .map(|s| sign_payload(&payload, s))
        .unwrap_or_default();

    let timestamp = Utc::now().timestamp();

    let response = client
        .post(webhook_url)
        .header("Content-Type", "application/json")
        .header("X-Webhook-Id", event.id.to_string())
        .header("X-Webhook-Timestamp", timestamp.to_string())
        .header("X-Webhook-Signature", signature)
        .body(payload)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "webhook delivery failed: {}",
            response.status()
        ));
    }

    Ok(())
}

async fn mark_delivered(pool: &PgPool, id: Uuid) -> anyhow::Result<()> {
    sqlx::query("UPDATE webhook_outbox SET status = 'delivered', processed_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

async fn schedule_retry(pool: &PgPool, id: Uuid, error: &str) -> anyhow::Result<()> {
    let event: Option<WebhookOutbox> = sqlx::query_as("SELECT * FROM webhook_outbox WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    let event = match event {
        Some(e) => e,
        None => return Ok(()),
    };

    let next_attempt = event.attempts + 1;

    if next_attempt >= event.max_attempts {
        sqlx::query("UPDATE webhook_outbox SET status = 'failed', last_error = $1 WHERE id = $2")
            .bind(error)
            .bind(id)
            .execute(pool)
            .await?;
    } else {
        let delay_secs = 2i64.pow(next_attempt as u32).min(3600);
        let jitter = rand::random::<i64>() % 1000;
        let next_attempt_at = Utc::now() + chrono::Duration::seconds(delay_secs + jitter / 1000);

        sqlx::query(
            "UPDATE webhook_outbox SET status = 'retrying', attempts = $1, next_attempt_at = $2, last_error = $3 WHERE id = $4",
        )
        .bind(next_attempt)
        .bind(next_attempt_at)
        .bind(error)
        .bind(id)
        .execute(pool)
        .await?;
    }

    Ok(())
}
