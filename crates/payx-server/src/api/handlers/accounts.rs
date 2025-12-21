use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::{
    Account, AccountResponse, CreateAccountRequest, Transaction, TransactionResponse,
};
use crate::error::{AppError, Result};
use crate::AppState;

#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    offset: Option<i64>,
    business_id: Option<Uuid>,
}

fn default_limit() -> i64 {
    50
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<AccountResponse>>> {
    let accounts: Vec<Account> = match query.business_id {
        Some(business_id) => {
            sqlx::query_as(
                "SELECT * FROM accounts WHERE business_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
            )
            .bind(business_id)
            .bind(query.limit)
            .bind(query.offset.unwrap_or(0))
            .fetch_all(&state.db)
            .await?
        }
        None => {
            sqlx::query_as(
                "SELECT * FROM accounts ORDER BY created_at DESC LIMIT $1 OFFSET $2",
            )
            .bind(query.limit)
            .bind(query.offset.unwrap_or(0))
            .fetch_all(&state.db)
            .await?
        }
    };

    Ok(Json(
        accounts.into_iter().map(AccountResponse::from).collect(),
    ))
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateAccountRequest>,
) -> Result<impl IntoResponse> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    let account: Account = sqlx::query_as(
        r#"
        INSERT INTO accounts (id, business_id, account_type, currency, balance, available_balance, version, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $5, 0, $6, $6)
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(req.business_id)
    .bind(&req.account_type)
    .bind(&req.currency)
    .bind(req.initial_balance)
    .bind(now)
    .fetch_one(&state.db)
    .await?;

    Ok((StatusCode::CREATED, Json(AccountResponse::from(account))))
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<AccountResponse>> {
    let account: Account = sqlx::query_as("SELECT * FROM accounts WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::AccountNotFound(id))?;

    Ok(Json(AccountResponse::from(account)))
}

#[derive(Deserialize)]
pub struct ListTransactionsQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    cursor: Option<Uuid>,
}

pub async fn list_transactions(
    State(state): State<Arc<AppState>>,
    Path(account_id): Path<Uuid>,
    Query(query): Query<ListTransactionsQuery>,
) -> Result<Json<Vec<TransactionResponse>>> {
    let _account: Account = sqlx::query_as("SELECT * FROM accounts WHERE id = $1")
        .bind(account_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::AccountNotFound(account_id))?;

    let transactions: Vec<Transaction> = match query.cursor {
        Some(cursor) => {
            sqlx::query_as(
                r#"
                SELECT * FROM transactions
                WHERE (source_account_id = $1 OR destination_account_id = $1)
                AND id < $2
                ORDER BY created_at DESC
                LIMIT $3
                "#,
            )
            .bind(account_id)
            .bind(cursor)
            .bind(query.limit)
            .fetch_all(&state.db)
            .await?
        }
        None => {
            sqlx::query_as(
                r#"
                SELECT * FROM transactions
                WHERE source_account_id = $1 OR destination_account_id = $1
                ORDER BY created_at DESC
                LIMIT $2
                "#,
            )
            .bind(account_id)
            .bind(query.limit)
            .fetch_all(&state.db)
            .await?
        }
    };

    Ok(Json(
        transactions
            .into_iter()
            .map(TransactionResponse::from)
            .collect(),
    ))
}
