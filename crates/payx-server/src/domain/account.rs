use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Account {
    pub id: Uuid,
    pub business_id: Uuid,
    pub account_type: String,
    pub currency: String,
    pub balance: Decimal,
    pub available_balance: Decimal,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAccountRequest {
    pub business_id: Uuid,
    #[serde(default = "default_account_type")]
    pub account_type: String,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(default)]
    pub initial_balance: Decimal,
}

fn default_account_type() -> String {
    "checking".into()
}

fn default_currency() -> String {
    "USD".into()
}

#[derive(Debug, Serialize)]
pub struct AccountResponse {
    pub id: Uuid,
    pub business_id: Uuid,
    pub account_type: String,
    pub currency: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub balance: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub available_balance: Decimal,
    pub created_at: DateTime<Utc>,
}

impl From<Account> for AccountResponse {
    fn from(a: Account) -> Self {
        Self {
            id: a.id,
            business_id: a.business_id,
            account_type: a.account_type,
            currency: a.currency,
            balance: a.balance,
            available_balance: a.available_balance,
            created_at: a.created_at,
        }
    }
}
