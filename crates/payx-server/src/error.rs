use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use rust_decimal::Decimal;
use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("insufficient funds: available {available}, requested {requested}")]
    InsufficientFunds {
        account_id: Uuid,
        available: Decimal,
        requested: Decimal,
    },

    #[error("account not found: {0}")]
    AccountNotFound(Uuid),

    #[error("business not found: {0}")]
    BusinessNotFound(Uuid),

    #[error("transaction not found: {0}")]
    TransactionNotFound(Uuid),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("currency mismatch: from {from_currency}, to {to_currency}")]
    CurrencyMismatch { from_currency: String, to_currency: String },

    #[error("idempotency conflict: existing transaction {existing_id}")]
    IdempotencyConflict {
        existing_id: Uuid,
        idempotency_key: String,
    },

    #[error("invalid api key")]
    InvalidApiKey,

    #[error("rate limit exceeded")]
    RateLimitExceeded,

    #[error("validation error: {0}")]
    Validation(String),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: ErrorBody,
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, details) = match &self {
            Self::InsufficientFunds {
                available,
                requested,
                ..
            } => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "insufficient_funds",
                Some(serde_json::json!({
                    "available": available.to_string(),
                    "requested": requested.to_string()
                })),
            ),
            Self::AccountNotFound(_) => (StatusCode::NOT_FOUND, "account_not_found", None),
            Self::BusinessNotFound(_) => (StatusCode::NOT_FOUND, "business_not_found", None),
            Self::TransactionNotFound(_) => (StatusCode::NOT_FOUND, "transaction_not_found", None),
            Self::NotFound(_) => (StatusCode::NOT_FOUND, "not_found", None),
            Self::CurrencyMismatch { .. } => (StatusCode::BAD_REQUEST, "currency_mismatch", None),
            Self::IdempotencyConflict { .. } => (StatusCode::CONFLICT, "idempotency_conflict", None),
            Self::InvalidApiKey => (StatusCode::UNAUTHORIZED, "invalid_api_key", None),
            Self::RateLimitExceeded => (StatusCode::TOO_MANY_REQUESTS, "rate_limit_exceeded", None),
            Self::Validation(_) => (StatusCode::BAD_REQUEST, "validation_error", None),
            Self::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "database_error", None),
            Self::Serialization(_) => (StatusCode::INTERNAL_SERVER_ERROR, "serialization_error", None),
            Self::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", None),
        };

        let body = ErrorResponse {
            error: ErrorBody {
                code,
                message: self.to_string(),
                details,
            },
        };

        (status, Json(body)).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
