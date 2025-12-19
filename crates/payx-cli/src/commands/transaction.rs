use anyhow::Result;
use clap::Subcommand;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tabled::Tabled;
use uuid::Uuid;

use crate::client::ApiClient;
use crate::config::Config;
use crate::output::{self, Format};

#[derive(Subcommand)]
pub enum Commands {
    /// List all transactions
    List {
        #[arg(long, help = "Filter by account ID")]
        account_id: Option<Uuid>,
        #[arg(long, default_value = "50")]
        limit: i64,
        #[arg(long)]
        offset: Option<i64>,
    },
    /// Credit funds to an account
    Credit {
        #[arg(long)]
        to: Uuid,
        #[arg(long)]
        amount: Decimal,
        #[arg(long, default_value = "USD")]
        currency: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long, help = "Idempotency key to prevent duplicates")]
        idempotency_key: Option<String>,
    },
    /// Debit funds from an account
    Debit {
        #[arg(long)]
        from: Uuid,
        #[arg(long)]
        amount: Decimal,
        #[arg(long, default_value = "USD")]
        currency: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long, help = "Idempotency key to prevent duplicates")]
        idempotency_key: Option<String>,
    },
    /// Transfer funds between accounts
    Transfer {
        #[arg(long)]
        from: Uuid,
        #[arg(long)]
        to: Uuid,
        #[arg(long)]
        amount: Decimal,
        #[arg(long, default_value = "USD")]
        currency: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long, help = "Idempotency key to prevent duplicates")]
        idempotency_key: Option<String>,
    },
    /// Get transaction details
    Get {
        #[arg(help = "Transaction ID")]
        id: Uuid,
    },
}

#[derive(Debug, Serialize)]
struct CreateRequest {
    #[serde(rename = "type")]
    tx_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_account_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    destination_account_id: Option<Uuid>,
    #[serde(with = "rust_decimal::serde::str")]
    amount: Decimal,
    currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Tabled)]
pub struct Transaction {
    pub id: Uuid,
    #[serde(rename = "type")]
    #[tabled(rename = "type")]
    pub tx_type: String,
    pub status: String,
    #[tabled(display_with = "display_uuid_option")]
    pub source_account_id: Option<Uuid>,
    #[tabled(display_with = "display_uuid_option")]
    pub destination_account_id: Option<Uuid>,
    #[tabled(display_with = "display_decimal")]
    pub amount: Decimal,
    pub currency: String,
}

fn display_decimal(d: &Decimal) -> String {
    d.to_string()
}

fn display_uuid_option(o: &Option<Uuid>) -> String {
    o.map(|u| u.to_string()).unwrap_or_else(|| "-".into())
}

pub async fn run(cmd: Commands, config: &Config, format: Format) -> Result<()> {
    let client = ApiClient::new(config);

    match cmd {
        Commands::List { account_id, limit, offset } => {
            let mut url = format!("/v1/transactions?limit={}", limit);
            if let Some(acc_id) = account_id {
                url.push_str(&format!("&account_id={}", acc_id));
            }
            if let Some(off) = offset {
                url.push_str(&format!("&offset={}", off));
            }
            let transactions: Vec<Transaction> = client.get(&url).await?;
            output::print_items(transactions, format);
        }
        Commands::Credit {
            to,
            amount,
            currency,
            description,
            idempotency_key,
        } => {
            let req = CreateRequest {
                tx_type: "credit".into(),
                source_account_id: None,
                destination_account_id: Some(to),
                amount,
                currency,
                description,
            };
            let txn: Transaction = client
                .post_with_idempotency("/v1/transactions", &req, idempotency_key.as_deref())
                .await?;
            output::print_created(txn, format);
        }
        Commands::Debit {
            from,
            amount,
            currency,
            description,
            idempotency_key,
        } => {
            let req = CreateRequest {
                tx_type: "debit".into(),
                source_account_id: Some(from),
                destination_account_id: None,
                amount,
                currency,
                description,
            };
            let txn: Transaction = client
                .post_with_idempotency("/v1/transactions", &req, idempotency_key.as_deref())
                .await?;
            output::print_created(txn, format);
        }
        Commands::Transfer {
            from,
            to,
            amount,
            currency,
            description,
            idempotency_key,
        } => {
            let req = CreateRequest {
                tx_type: "transfer".into(),
                source_account_id: Some(from),
                destination_account_id: Some(to),
                amount,
                currency,
                description,
            };
            let txn: Transaction = client
                .post_with_idempotency("/v1/transactions", &req, idempotency_key.as_deref())
                .await?;
            output::print_created(txn, format);
        }
        Commands::Get { id } => {
            let txn: Transaction = client.get(&format!("/v1/transactions/{}", id)).await?;
            output::print_item(txn, format);
        }
    }

    Ok(())
}
