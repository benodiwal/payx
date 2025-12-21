use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use chrono::Utc;
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::{
    Account, CreateTransactionRequest, Transaction, TransactionResponse, TransactionStatus,
    TransactionType, WebhookPayload,
};
use crate::error::{AppError, Result};
use crate::AppState;

#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    offset: Option<i64>,
    account_id: Option<Uuid>,
}

fn default_limit() -> i64 {
    50
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<TransactionResponse>>> {
    let transactions: Vec<Transaction> = match query.account_id {
        Some(account_id) => {
            sqlx::query_as(
                r#"
                SELECT * FROM transactions
                WHERE source_account_id = $1 OR destination_account_id = $1
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(account_id)
            .bind(query.limit)
            .bind(query.offset.unwrap_or(0))
            .fetch_all(&state.db)
            .await?
        }
        None => {
            sqlx::query_as("SELECT * FROM transactions ORDER BY created_at DESC LIMIT $1 OFFSET $2")
                .bind(query.limit)
                .bind(query.offset.unwrap_or(0))
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

pub async fn create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateTransactionRequest>,
) -> Result<impl IntoResponse> {
    let idempotency_key = headers
        .get("idempotency-key")
        .and_then(|h| h.to_str().ok())
        .map(String::from);

    if req.amount <= Decimal::ZERO {
        return Err(AppError::Validation("amount must be positive".into()));
    }

    if let Some(ref key) = idempotency_key {
        if let Some(existing) = find_by_idempotency_key(&state, key).await? {
            return Ok((StatusCode::OK, Json(TransactionResponse::from(existing))));
        }
    }

    let transaction = match req.tx_type {
        TransactionType::Credit => execute_credit(&state, &req, idempotency_key.as_deref()).await?,
        TransactionType::Debit => execute_debit(&state, &req, idempotency_key.as_deref()).await?,
        TransactionType::Transfer => {
            execute_transfer(&state, &req, idempotency_key.as_deref()).await?
        }
    };

    Ok((
        StatusCode::CREATED,
        Json(TransactionResponse::from(transaction)),
    ))
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<TransactionResponse>> {
    let transaction: Transaction = sqlx::query_as("SELECT * FROM transactions WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::TransactionNotFound(id))?;

    Ok(Json(TransactionResponse::from(transaction)))
}

async fn find_by_idempotency_key(state: &AppState, key: &str) -> Result<Option<Transaction>> {
    let txn: Option<Transaction> =
        sqlx::query_as("SELECT * FROM transactions WHERE idempotency_key = $1")
            .bind(key)
            .fetch_optional(&state.db)
            .await?;
    Ok(txn)
}

async fn execute_credit(
    state: &AppState,
    req: &CreateTransactionRequest,
    idempotency_key: Option<&str>,
) -> Result<Transaction> {
    let dest_id = req
        .destination_account_id
        .ok_or_else(|| AppError::Validation("destination_account_id required for credit".into()))?;

    let mut tx = state.db.begin().await?;
    let now = Utc::now();
    let txn_id = Uuid::new_v4();

    let dest: Account = sqlx::query_as("SELECT * FROM accounts WHERE id = $1 FOR UPDATE")
        .bind(dest_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(AppError::AccountNotFound(dest_id))?;

    if dest.currency != req.currency {
        return Err(AppError::CurrencyMismatch {
            from_currency: req.currency.clone(),
            to_currency: dest.currency,
        });
    }

    let new_balance = dest.balance + req.amount;

    sqlx::query("UPDATE accounts SET balance = $1, available_balance = $1, version = version + 1, updated_at = $2 WHERE id = $3")
        .bind(new_balance)
        .bind(now)
        .bind(dest_id)
        .execute(&mut *tx)
        .await?;

    let transaction: Transaction = sqlx::query_as(
        r#"
        INSERT INTO transactions (id, idempotency_key, type, status, destination_account_id, amount, currency, description, metadata, created_at, completed_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $10)
        RETURNING *
        "#,
    )
    .bind(txn_id)
    .bind(idempotency_key)
    .bind(TransactionType::Credit)
    .bind(TransactionStatus::Completed)
    .bind(dest_id)
    .bind(req.amount)
    .bind(&req.currency)
    .bind(&req.description)
    .bind(&req.metadata)
    .bind(now)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO ledger_entries (id, transaction_id, account_id, entry_type, amount, balance_after, created_at)
        VALUES ($1, $2, $3, 'credit', $4, $5, $6)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(txn_id)
    .bind(dest_id)
    .bind(req.amount)
    .bind(new_balance)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    enqueue_webhook(
        &mut tx,
        dest.business_id,
        "transaction.completed",
        &transaction,
    )
    .await?;

    tx.commit().await?;
    Ok(transaction)
}

async fn execute_debit(
    state: &AppState,
    req: &CreateTransactionRequest,
    idempotency_key: Option<&str>,
) -> Result<Transaction> {
    let source_id = req
        .source_account_id
        .ok_or_else(|| AppError::Validation("source_account_id required for debit".into()))?;

    let mut tx = state.db.begin().await?;
    let now = Utc::now();
    let txn_id = Uuid::new_v4();

    let source: Account = sqlx::query_as("SELECT * FROM accounts WHERE id = $1 FOR UPDATE")
        .bind(source_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(AppError::AccountNotFound(source_id))?;

    if source.currency != req.currency {
        return Err(AppError::CurrencyMismatch {
            from_currency: source.currency,
            to_currency: req.currency.clone(),
        });
    }

    if source.available_balance < req.amount {
        return Err(AppError::InsufficientFunds {
            account_id: source_id,
            available: source.available_balance,
            requested: req.amount,
        });
    }

    let new_balance = source.balance - req.amount;

    sqlx::query("UPDATE accounts SET balance = $1, available_balance = $1, version = version + 1, updated_at = $2 WHERE id = $3")
        .bind(new_balance)
        .bind(now)
        .bind(source_id)
        .execute(&mut *tx)
        .await?;

    let transaction: Transaction = sqlx::query_as(
        r#"
        INSERT INTO transactions (id, idempotency_key, type, status, source_account_id, amount, currency, description, metadata, created_at, completed_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $10)
        RETURNING *
        "#,
    )
    .bind(txn_id)
    .bind(idempotency_key)
    .bind(TransactionType::Debit)
    .bind(TransactionStatus::Completed)
    .bind(source_id)
    .bind(req.amount)
    .bind(&req.currency)
    .bind(&req.description)
    .bind(&req.metadata)
    .bind(now)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO ledger_entries (id, transaction_id, account_id, entry_type, amount, balance_after, created_at)
        VALUES ($1, $2, $3, 'debit', $4, $5, $6)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(txn_id)
    .bind(source_id)
    .bind(req.amount)
    .bind(new_balance)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    enqueue_webhook(
        &mut tx,
        source.business_id,
        "transaction.completed",
        &transaction,
    )
    .await?;

    tx.commit().await?;
    Ok(transaction)
}

async fn execute_transfer(
    state: &AppState,
    req: &CreateTransactionRequest,
    idempotency_key: Option<&str>,
) -> Result<Transaction> {
    let source_id = req
        .source_account_id
        .ok_or_else(|| AppError::Validation("source_account_id required for transfer".into()))?;
    let dest_id = req.destination_account_id.ok_or_else(|| {
        AppError::Validation("destination_account_id required for transfer".into())
    })?;

    let mut tx = state.db.begin().await?;
    let now = Utc::now();
    let txn_id = Uuid::new_v4();

    // Lock in consistent order to prevent deadlocks
    let (first_id, second_id) = if source_id < dest_id {
        (source_id, dest_id)
    } else {
        (dest_id, source_id)
    };

    let first: Account = sqlx::query_as("SELECT * FROM accounts WHERE id = $1 FOR UPDATE")
        .bind(first_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(AppError::AccountNotFound(first_id))?;

    let second: Account = sqlx::query_as("SELECT * FROM accounts WHERE id = $1 FOR UPDATE")
        .bind(second_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(AppError::AccountNotFound(second_id))?;

    let (source, dest) = if first_id == source_id {
        (first, second)
    } else {
        (second, first)
    };

    if source.currency != req.currency {
        return Err(AppError::CurrencyMismatch {
            from_currency: source.currency,
            to_currency: req.currency.clone(),
        });
    }

    if dest.currency != req.currency {
        return Err(AppError::CurrencyMismatch {
            from_currency: req.currency.clone(),
            to_currency: dest.currency,
        });
    }

    if source.available_balance < req.amount {
        return Err(AppError::InsufficientFunds {
            account_id: source_id,
            available: source.available_balance,
            requested: req.amount,
        });
    }

    let source_new_balance = source.balance - req.amount;
    let dest_new_balance = dest.balance + req.amount;

    sqlx::query("UPDATE accounts SET balance = $1, available_balance = $1, version = version + 1, updated_at = $2 WHERE id = $3")
        .bind(source_new_balance)
        .bind(now)
        .bind(source_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE accounts SET balance = $1, available_balance = $1, version = version + 1, updated_at = $2 WHERE id = $3")
        .bind(dest_new_balance)
        .bind(now)
        .bind(dest_id)
        .execute(&mut *tx)
        .await?;

    let transaction: Transaction = sqlx::query_as(
        r#"
        INSERT INTO transactions (id, idempotency_key, type, status, source_account_id, destination_account_id, amount, currency, description, metadata, created_at, completed_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $11)
        RETURNING *
        "#,
    )
    .bind(txn_id)
    .bind(idempotency_key)
    .bind(TransactionType::Transfer)
    .bind(TransactionStatus::Completed)
    .bind(source_id)
    .bind(dest_id)
    .bind(req.amount)
    .bind(&req.currency)
    .bind(&req.description)
    .bind(&req.metadata)
    .bind(now)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO ledger_entries (id, transaction_id, account_id, entry_type, amount, balance_after, created_at)
        VALUES ($1, $2, $3, 'debit', $4, $5, $6)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(txn_id)
    .bind(source_id)
    .bind(req.amount)
    .bind(source_new_balance)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO ledger_entries (id, transaction_id, account_id, entry_type, amount, balance_after, created_at)
        VALUES ($1, $2, $3, 'credit', $4, $5, $6)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(txn_id)
    .bind(dest_id)
    .bind(req.amount)
    .bind(dest_new_balance)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    enqueue_webhook(
        &mut tx,
        source.business_id,
        "transaction.completed",
        &transaction,
    )
    .await?;
    if source.business_id != dest.business_id {
        enqueue_webhook(
            &mut tx,
            dest.business_id,
            "transaction.completed",
            &transaction,
        )
        .await?;
    }

    tx.commit().await?;
    Ok(transaction)
}

async fn enqueue_webhook(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    business_id: Uuid,
    event_type: &str,
    transaction: &Transaction,
) -> Result<()> {
    let payload = WebhookPayload::new(event_type, serde_json::to_value(transaction)?);

    sqlx::query(
        r#"
        INSERT INTO webhook_outbox (id, business_id, event_type, payload, status, attempts, max_attempts, next_attempt_at, created_at)
        VALUES ($1, $2, $3, $4, 'pending', 0, 5, NOW(), NOW())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(business_id)
    .bind(event_type)
    .bind(serde_json::to_value(&payload)?)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
