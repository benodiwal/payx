use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use chrono::{Duration, DurationRound, Utc};

use crate::api::middleware::auth::AuthContext;
use crate::error::AppError;
use crate::AppState;

pub async fn middleware(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let auth = req
        .extensions()
        .get::<AuthContext>()
        .expect("auth middleware must run first");

    let window_start = Utc::now()
        .duration_trunc(Duration::minutes(1))
        .expect("valid truncation");

    let result = sqlx::query_scalar::<_, i32>(
        r#"
        INSERT INTO rate_limit_windows (api_key_id, window_start, request_count)
        VALUES ($1, $2, 1)
        ON CONFLICT (api_key_id, window_start)
        DO UPDATE SET request_count = rate_limit_windows.request_count + 1
        RETURNING request_count
        "#,
    )
    .bind(auth.api_key.id)
    .bind(window_start)
    .fetch_one(&state.db)
    .await?;

    if result > auth.api_key.rate_limit_per_minute {
        return Err(AppError::RateLimitExceeded);
    }

    Ok(next.run(req).await)
}
