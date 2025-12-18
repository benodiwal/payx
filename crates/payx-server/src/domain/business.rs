use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Business {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub webhook_url: Option<String>,
    #[serde(skip)]
    pub webhook_secret: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBusinessRequest {
    pub name: String,
    pub email: String,
    pub webhook_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBusinessRequest {
    pub name: Option<String>,
    pub webhook_url: Option<String>,
}
