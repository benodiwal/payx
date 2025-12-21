use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use rand::RngCore;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::{Business, CreateBusinessRequest, GeneratedApiKey, UpdateBusinessRequest};
use crate::error::{AppError, Result};
use crate::AppState;

#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    offset: Option<i64>,
}

fn default_limit() -> i64 {
    50
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<Business>>> {
    let businesses: Vec<Business> =
        sqlx::query_as("SELECT * FROM businesses ORDER BY created_at DESC LIMIT $1 OFFSET $2")
            .bind(query.limit)
            .bind(query.offset.unwrap_or(0))
            .fetch_all(&state.db)
            .await?;

    Ok(Json(businesses))
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateBusinessRequest>,
) -> Result<impl IntoResponse> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    let mut secret_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut secret_bytes);
    let webhook_secret = URL_SAFE_NO_PAD.encode(secret_bytes);

    let business: Business = sqlx::query_as(
        r#"
        INSERT INTO businesses (id, name, email, webhook_url, webhook_secret, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $6)
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.email)
    .bind(&req.webhook_url)
    .bind(&webhook_secret)
    .bind(now)
    .fetch_one(&state.db)
    .await?;

    let (api_key, generated) = crate::domain::ApiKey::generate(business.id);

    sqlx::query(
        r#"
        INSERT INTO api_keys (id, business_id, key_hash, key_prefix, rate_limit_per_minute, created_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(api_key.id)
    .bind(api_key.business_id)
    .bind(&api_key.key_hash)
    .bind(&api_key.key_prefix)
    .bind(api_key.rate_limit_per_minute)
    .bind(api_key.created_at)
    .execute(&state.db)
    .await?;

    #[derive(serde::Serialize)]
    struct Response {
        business: Business,
        api_key: GeneratedApiKey,
        webhook_secret: String,
    }

    Ok((
        StatusCode::CREATED,
        Json(Response {
            business,
            api_key: generated,
            webhook_secret,
        }),
    ))
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Business>> {
    let business: Business = sqlx::query_as("SELECT * FROM businesses WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::BusinessNotFound(id))?;

    Ok(Json(business))
}

pub async fn update(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateBusinessRequest>,
) -> Result<Json<Business>> {
    let business: Business = sqlx::query_as(
        r#"
        UPDATE businesses
        SET name = COALESCE($2, name),
            webhook_url = COALESCE($3, webhook_url),
            updated_at = $4
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.webhook_url)
    .bind(Utc::now())
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::BusinessNotFound(id))?;

    Ok(Json(business))
}
