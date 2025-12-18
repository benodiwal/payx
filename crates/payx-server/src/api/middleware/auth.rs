use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, Request};
use axum::middleware::Next;
use axum::response::Response;
use chrono::Utc;

use crate::domain::ApiKey;
use crate::error::AppError;
use crate::AppState;

#[derive(Clone)]
pub struct AuthContext {
    pub api_key: ApiKey,
}

pub async fn middleware(
    State(state): State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(AppError::InvalidApiKey)?;

    let key = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::InvalidApiKey)?;

    if key.len() < 12 {
        return Err(AppError::InvalidApiKey);
    }
    let prefix = &key[..12];

    let api_key: ApiKey = sqlx::query_as(
        "SELECT * FROM api_keys WHERE key_prefix = $1 AND revoked_at IS NULL",
    )
    .bind(prefix)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::InvalidApiKey)?;

    if !api_key.is_valid() || !api_key.verify(key) {
        return Err(AppError::InvalidApiKey);
    }

    sqlx::query("UPDATE api_keys SET last_used_at = $1 WHERE id = $2")
        .bind(Utc::now())
        .bind(api_key.id)
        .execute(&state.db)
        .await?;

    req.extensions_mut().insert(AuthContext { api_key });

    Ok(next.run(req).await)
}
