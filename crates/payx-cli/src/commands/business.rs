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
    /// List all businesses
    List {
        #[arg(long, default_value = "50")]
        limit: i64,
        #[arg(long)]
        offset: Option<i64>,
    },
    /// Create a new business
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        email: String,
        #[arg(long)]
        webhook_url: Option<String>,
    },
    /// Get business details
    Get {
        #[arg(help = "Business ID")]
        id: Uuid,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateRequest {
    name: String,
    email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    webhook_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Tabled)]
pub struct Business {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    #[tabled(display_with = "display_option")]
    pub webhook_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiKey {
    id: Uuid,
    key: String,
    prefix: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateResponse {
    business: Business,
    api_key: ApiKey,
    webhook_secret: String,
}

fn display_option(o: &Option<String>) -> String {
    o.clone().unwrap_or_else(|| "-".into())
}

pub async fn run(cmd: Commands, config: &Config, format: Format) -> Result<()> {
    let client = ApiClient::new(config);

    match cmd {
        Commands::List { limit, offset } => {
            let mut url = format!("/v1/businesses?limit={}", limit);
            if let Some(off) = offset {
                url.push_str(&format!("&offset={}", off));
            }
            let businesses: Vec<Business> = client.get(&url).await?;
            output::print_items(businesses, format);
        }
        Commands::Create {
            name,
            email,
            webhook_url,
        } => {
            let req = CreateRequest {
                name,
                email,
                webhook_url,
            };
            let resp: CreateResponse = client.post_no_auth("/v1/businesses", &req).await?;

            match format {
                Format::Json => output::print_json(&resp),
                Format::Table => {
                    output::print_success("Business created");
                    output::print_single(resp.business);
                    println!();
                    println!("API Key (save this, it won't be shown again):");
                    println!("  {}", resp.api_key.key);
                    println!();
                    println!("Webhook Secret:");
                    println!("  {}", resp.webhook_secret);
                    println!();
                    println!("To configure the CLI:");
                    println!("  payx config set --api-key {}", resp.api_key.key);
                }
            }
        }
        Commands::Get { id } => {
            let business: Business = client.get(&format!("/v1/businesses/{}", id)).await?;
            output::print_item(business, format);
        }
    }

    Ok(())
}
