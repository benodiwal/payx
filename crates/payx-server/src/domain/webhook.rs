use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WebhookOutbox {
    pub id: Uuid,
    pub business_id: Uuid,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub next_attempt_at: DateTime<Utc>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub id: Uuid,
    pub event_type: String,
    pub created_at: DateTime<Utc>,
    pub data: serde_json::Value,
}

impl WebhookPayload {
    pub fn new(event_type: &str, data: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type: event_type.to_string(),
            created_at: Utc::now(),
            data,
        }
    }
}

pub fn sign_payload(payload: &[u8], secret: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("valid key");
    mac.update(payload);
    let result = mac.finalize();
    format!("sha256={}", hex::encode(result.into_bytes()))
}

pub fn verify_signature(payload: &[u8], secret: &str, signature: &str) -> bool {
    let expected = sign_payload(payload, secret);
    constant_time_eq(expected.as_bytes(), signature.as_bytes())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).fold(0, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[derive(Debug, Deserialize)]
pub struct CreateWebhookEndpointRequest {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWebhookEndpointRequest {
    pub url: Option<String>,
}
