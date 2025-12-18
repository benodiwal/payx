use std::sync::Arc;

use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::middleware::auth::AuthContext;
use crate::domain::{Business, CreateWebhookEndpointRequest, UpdateWebhookEndpointRequest, WebhookOutbox};
use crate::error::{AppError, Result};
use crate::AppState;

#[derive(Serialize)]
pub struct WebhookEndpointResponse {
    pub id: Uuid,
    pub url: Option<String>,
    pub secret: String,
}

pub async fn create_endpoint(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreateWebhookEndpointRequest>,
) -> Result<impl IntoResponse> {
    let mut secret_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut secret_bytes);
    let webhook_secret = URL_SAFE_NO_PAD.encode(secret_bytes);

    let business: Business = sqlx::query_as(
        r#"
        UPDATE businesses
        SET webhook_url = $1, webhook_secret = $2, updated_at = $3
        WHERE id = $4
        RETURNING *
        "#,
    )
    .bind(&req.url)
    .bind(&webhook_secret)
    .bind(Utc::now())
    .bind(auth.api_key.business_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::BusinessNotFound(auth.api_key.business_id))?;

    Ok((
        StatusCode::CREATED,
        Json(WebhookEndpointResponse {
            id: business.id,
            url: business.webhook_url,
            secret: webhook_secret,
        }),
    ))
}

pub async fn update_endpoint(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(_id): Path<Uuid>,
    Json(req): Json<UpdateWebhookEndpointRequest>,
) -> Result<Json<WebhookEndpointResponse>> {
    let business: Business = sqlx::query_as(
        r#"
        UPDATE businesses
        SET webhook_url = COALESCE($1, webhook_url), updated_at = $2
        WHERE id = $3
        RETURNING *
        "#,
    )
    .bind(&req.url)
    .bind(Utc::now())
    .bind(auth.api_key.business_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::BusinessNotFound(auth.api_key.business_id))?;

    Ok(Json(WebhookEndpointResponse {
        id: business.id,
        url: business.webhook_url,
        secret: business.webhook_secret.unwrap_or_default(),
    }))
}

pub async fn delete_endpoint(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(_id): Path<Uuid>,
) -> Result<StatusCode> {
    sqlx::query(
        r#"
        UPDATE businesses
        SET webhook_url = NULL, webhook_secret = NULL, updated_at = $1
        WHERE id = $2
        "#,
    )
    .bind(Utc::now())
    .bind(auth.api_key.business_id)
    .execute(&state.db)
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct ListDeliveriesQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    offset: Option<i64>,
    status: Option<String>,
}

fn default_limit() -> i64 {
    50
}

#[derive(Serialize)]
pub struct WebhookDeliveryResponse {
    pub id: Uuid,
    pub event_type: String,
    pub status: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub next_attempt_at: DateTime<Utc>,
}

impl From<WebhookOutbox> for WebhookDeliveryResponse {
    fn from(w: WebhookOutbox) -> Self {
        Self {
            id: w.id,
            event_type: w.event_type,
            status: w.status,
            attempts: w.attempts,
            max_attempts: w.max_attempts,
            last_error: w.last_error,
            created_at: w.created_at,
            processed_at: w.processed_at,
            next_attempt_at: w.next_attempt_at,
        }
    }
}

pub async fn list_deliveries(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Query(query): Query<ListDeliveriesQuery>,
) -> Result<Json<Vec<WebhookDeliveryResponse>>> {
    let deliveries: Vec<WebhookOutbox> = match &query.status {
        Some(status) => {
            sqlx::query_as(
                r#"
                SELECT * FROM webhook_outbox
                WHERE business_id = $1 AND status = $2
                ORDER BY created_at DESC
                LIMIT $3 OFFSET $4
                "#,
            )
            .bind(auth.api_key.business_id)
            .bind(status)
            .bind(query.limit)
            .bind(query.offset.unwrap_or(0))
            .fetch_all(&state.db)
            .await?
        }
        None => {
            sqlx::query_as(
                r#"
                SELECT * FROM webhook_outbox
                WHERE business_id = $1
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(auth.api_key.business_id)
            .bind(query.limit)
            .bind(query.offset.unwrap_or(0))
            .fetch_all(&state.db)
            .await?
        }
    };

    Ok(Json(deliveries.into_iter().map(WebhookDeliveryResponse::from).collect()))
}

pub async fn get_delivery(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<WebhookDeliveryResponse>> {
    let delivery: WebhookOutbox = sqlx::query_as(
        "SELECT * FROM webhook_outbox WHERE id = $1 AND business_id = $2",
    )
    .bind(id)
    .bind(auth.api_key.business_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Webhook delivery not found".into()))?;

    Ok(Json(WebhookDeliveryResponse::from(delivery)))
}

pub async fn retry_delivery(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<WebhookDeliveryResponse>> {
    let delivery: WebhookOutbox = sqlx::query_as(
        r#"
        UPDATE webhook_outbox
        SET status = 'pending', attempts = 0, next_attempt_at = NOW(), last_error = NULL
        WHERE id = $1 AND business_id = $2 AND status = 'failed'
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(auth.api_key.business_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Webhook delivery not found or not in failed status".into()))?;

    Ok(Json(WebhookDeliveryResponse::from(delivery)))
}
