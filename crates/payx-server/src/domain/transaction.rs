use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Credit,
    Debit,
    Transfer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TransactionStatus {
    Pending,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Transaction {
    pub id: Uuid,
    pub idempotency_key: Option<String>,
    #[sqlx(rename = "type")]
    pub tx_type: TransactionType,
    pub status: TransactionStatus,
    pub source_account_id: Option<Uuid>,
    pub destination_account_id: Option<Uuid>,
    pub amount: Decimal,
    pub currency: String,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTransactionRequest {
    #[serde(rename = "type")]
    pub tx_type: TransactionType,
    pub source_account_id: Option<Uuid>,
    pub destination_account_id: Option<Uuid>,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    pub currency: String,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct TransactionResponse {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub tx_type: TransactionType,
    pub status: TransactionStatus,
    pub source_account_id: Option<Uuid>,
    pub destination_account_id: Option<Uuid>,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    pub currency: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl From<Transaction> for TransactionResponse {
    fn from(t: Transaction) -> Self {
        Self {
            id: t.id,
            tx_type: t.tx_type,
            status: t.status,
            source_account_id: t.source_account_id,
            destination_account_id: t.destination_account_id,
            amount: t.amount,
            currency: t.currency,
            description: t.description,
            created_at: t.created_at,
            completed_at: t.completed_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct LedgerEntry {
    pub id: Uuid,
    pub transaction_id: Uuid,
    pub account_id: Uuid,
    pub entry_type: String,
    pub amount: Decimal,
    pub balance_after: Decimal,
    pub created_at: DateTime<Utc>,
}
