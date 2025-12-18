use std::sync::Arc;
use std::time::Duration;

use axum::http::StatusCode;
use axum::middleware::from_fn_with_state;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use serde_json::json;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::api::handlers::{accounts, businesses, health, transactions, webhooks};
use crate::api::middleware::{auth, rate_limit};
use crate::AppState;

async fn fallback() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": {
                "code": "not_found",
                "message": "The requested endpoint does not exist"
            }
        })),
    )
}

pub fn build(state: Arc<AppState>) -> Router {
    let protected = Router::new()
        .route("/businesses", get(businesses::list))
        .route("/businesses/:id", get(businesses::get))
        .route("/businesses/:id", put(businesses::update))
        .route("/accounts", get(accounts::list))
        .route("/accounts", post(accounts::create))
        .route("/accounts/:id", get(accounts::get))
        .route("/accounts/:id/transactions", get(accounts::list_transactions))
        .route("/transactions", get(transactions::list))
        .route("/transactions", post(transactions::create))
        .route("/transactions/:id", get(transactions::get))
        .route("/webhooks/endpoints", post(webhooks::create_endpoint))
        .route("/webhooks/endpoints/:id", put(webhooks::update_endpoint))
        .route("/webhooks/endpoints/:id", delete(webhooks::delete_endpoint))
        .route("/webhooks/deliveries", get(webhooks::list_deliveries))
        .route("/webhooks/deliveries/:id", get(webhooks::get_delivery))
        .route("/webhooks/deliveries/:id/retry", post(webhooks::retry_delivery))
        .layer(from_fn_with_state(state.clone(), rate_limit::middleware))
        .layer(from_fn_with_state(state.clone(), auth::middleware));

    let public = Router::new()
        .route("/health", get(health::health))
        .route("/ready", get(health::ready))
        .route("/v1/businesses", post(businesses::create));

    let api = Router::new()
        .nest("/v1", protected)
        .merge(public)
        .fallback(fallback);

    api.with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                .layer(PropagateRequestIdLayer::x_request_id())
                .layer(TraceLayer::new_for_http())
                .layer(TimeoutLayer::new(Duration::from_secs(30)))
                .layer(CompressionLayer::new())
                .layer(CorsLayer::permissive()),
        )
}
