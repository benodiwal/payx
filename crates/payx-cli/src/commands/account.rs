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
    /// List all accounts
    List {
        #[arg(long, help = "Filter by business ID")]
        business_id: Option<Uuid>,
        #[arg(long, default_value = "50")]
        limit: i64,
        #[arg(long)]
        offset: Option<i64>,
    },
    /// Create a new account
    Create {
        #[arg(long)]
        business_id: Uuid,
        #[arg(long, default_value = "USD")]
        currency: String,
        #[arg(long, default_value = "0")]
        balance: Decimal,
    },
    /// Get account details
    Get {
        #[arg(help = "Account ID")]
        id: Uuid,
    },
    /// List account transactions
    Transactions {
        #[arg(help = "Account ID")]
        id: Uuid,
        #[arg(long, default_value = "20")]
        limit: i64,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateRequest {
    business_id: Uuid,
    currency: String,
    #[serde(with = "rust_decimal::serde::str")]
    initial_balance: Decimal,
}

#[derive(Debug, Serialize, Deserialize, Tabled)]
pub struct Account {
    pub id: Uuid,
    pub business_id: Uuid,
    pub currency: String,
    #[tabled(display_with = "display_decimal")]
    pub balance: Decimal,
    #[tabled(display_with = "display_decimal")]
    pub available_balance: Decimal,
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
        Commands::List {
            business_id,
            limit,
            offset,
        } => {
            let mut url = format!("/v1/accounts?limit={}", limit);
            if let Some(biz_id) = business_id {
                url.push_str(&format!("&business_id={}", biz_id));
            }
            if let Some(off) = offset {
                url.push_str(&format!("&offset={}", off));
            }
            let accounts: Vec<Account> = client.get(&url).await?;
            output::print_items(accounts, format);
        }
        Commands::Create {
            business_id,
            currency,
            balance,
        } => {
            let req = CreateRequest {
                business_id,
                currency,
                initial_balance: balance,
            };
            let account: Account = client.post("/v1/accounts", &req).await?;
            output::print_created(account, format);
        }
        Commands::Get { id } => {
            let account: Account = client.get(&format!("/v1/accounts/{}", id)).await?;
            output::print_item(account, format);
        }
        Commands::Transactions { id, limit } => {
            let txns: Vec<Transaction> = client
                .get(&format!("/v1/accounts/{}/transactions?limit={}", id, limit))
                .await?;
            output::print_items(txns, format);
        }
    }

    Ok(())
}
