use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use tabled::Tabled;
use uuid::Uuid;

use crate::client::ApiClient;
use crate::config::Config;
use crate::output::{self, Format};

#[derive(Subcommand)]
pub enum Commands {
    /// List webhook deliveries
    List {
        #[arg(long, default_value = "50")]
        limit: i64,
        #[arg(long)]
        offset: Option<i64>,
        #[arg(long, help = "Filter by status: pending, retrying, delivered, failed")]
        status: Option<String>,
    },
    /// Get webhook delivery details
    Get {
        #[arg(help = "Delivery ID")]
        id: Uuid,
    },
    /// Retry a failed webhook delivery
    Retry {
        #[arg(help = "Delivery ID")]
        id: Uuid,
    },
}

#[derive(Debug, Serialize, Deserialize, Tabled)]
pub struct WebhookDelivery {
    pub id: Uuid,
    pub event_type: String,
    pub status: String,
    pub attempts: i32,
    pub max_attempts: i32,
    #[tabled(display_with = "display_option")]
    pub last_error: Option<String>,
}

fn display_option(o: &Option<String>) -> String {
    o.clone().unwrap_or_else(|| "-".into())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookDeliveryDetail {
    pub id: Uuid,
    pub event_type: String,
    pub status: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub last_error: Option<String>,
    pub created_at: String,
    pub processed_at: Option<String>,
    pub next_attempt_at: String,
}

pub async fn run(cmd: Commands, config: &Config, format: Format) -> Result<()> {
    let client = ApiClient::new(config);

    match cmd {
        Commands::List {
            limit,
            offset,
            status,
        } => {
            let mut url = format!("/v1/webhooks/deliveries?limit={}", limit);
            if let Some(off) = offset {
                url.push_str(&format!("&offset={}", off));
            }
            if let Some(s) = status {
                url.push_str(&format!("&status={}", s));
            }
            let deliveries: Vec<WebhookDelivery> = client.get(&url).await?;
            output::print_items(deliveries, format);
        }
        Commands::Get { id } => {
            let delivery: WebhookDeliveryDetail = client
                .get(&format!("/v1/webhooks/deliveries/{}", id))
                .await?;
            match format {
                Format::Json => output::print_json(&delivery),
                Format::Table => {
                    println!("ID:             {}", delivery.id);
                    println!("Event Type:     {}", delivery.event_type);
                    println!("Status:         {}", delivery.status);
                    println!(
                        "Attempts:       {}/{}",
                        delivery.attempts, delivery.max_attempts
                    );
                    println!("Created:        {}", delivery.created_at);
                    if let Some(processed) = &delivery.processed_at {
                        println!("Processed:      {}", processed);
                    }
                    println!("Next Attempt:   {}", delivery.next_attempt_at);
                    if let Some(error) = &delivery.last_error {
                        println!("Last Error:     {}", error);
                    }
                }
            }
        }
        Commands::Retry { id } => {
            let delivery: WebhookDelivery = client
                .post(&format!("/v1/webhooks/deliveries/{}/retry", id), &())
                .await?;
            output::print_success("Webhook delivery queued for retry");
            output::print_single(delivery);
        }
    }

    Ok(())
}
