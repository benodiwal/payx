mod client;
mod commands;
mod config;
mod output;

use anyhow::Result;
use clap::{Parser, Subcommand};
use commands::{account, business, transaction, webhook};

#[derive(Parser)]
#[command(name = "payx")]
#[command(about = "PayX CLI - Interact with the PayX transaction service")]
#[command(version)]
struct Cli {
    #[arg(long, global = true, help = "API server URL")]
    server: Option<String>,

    #[arg(long, global = true, help = "API key for authentication")]
    api_key: Option<String>,

    #[arg(long, global = true, help = "Output format", default_value = "table")]
    format: output::Format,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Configure CLI settings
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Manage businesses
    Business {
        #[command(subcommand)]
        command: business::Commands,
    },
    /// Manage accounts
    Account {
        #[command(subcommand)]
        command: account::Commands,
    },
    /// Manage transactions
    #[command(alias = "tx")]
    Transaction {
        #[command(subcommand)]
        command: transaction::Commands,
    },
    /// Manage webhooks
    #[command(alias = "wh")]
    Webhook {
        #[command(subcommand)]
        command: webhook::Commands,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Set configuration values
    Set {
        #[arg(long)]
        server: Option<String>,
        #[arg(long)]
        api_key: Option<String>,
    },
    /// Show current configuration
    Show,
    /// Get config file path
    Path,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut cfg = config::Config::load()?;

    if let Some(server) = &cli.server {
        cfg.server = server.clone();
    }
    if let Some(api_key) = &cli.api_key {
        cfg.api_key = Some(api_key.clone());
    }

    match cli.command {
        Commands::Config { command } => match command {
            ConfigCommands::Set { server, api_key } => {
                if let Some(s) = server {
                    cfg.server = s;
                }
                if let Some(k) = api_key {
                    cfg.api_key = Some(k);
                }
                cfg.save()?;
                println!("Configuration saved");
            }
            ConfigCommands::Show => {
                println!("Server: {}", cfg.server);
                println!(
                    "API Key: {}",
                    cfg.api_key
                        .as_ref()
                        .map(|k| format!("{}...", &k[..12.min(k.len())]))
                        .unwrap_or_else(|| "(not set)".into())
                );
            }
            ConfigCommands::Path => {
                println!("{}", config::config_path()?.display());
            }
        },
        Commands::Business { command } => {
            business::run(command, &cfg, cli.format).await?;
        }
        Commands::Account { command } => {
            account::run(command, &cfg, cli.format).await?;
        }
        Commands::Transaction { command } => {
            transaction::run(command, &cfg, cli.format).await?;
        }
        Commands::Webhook { command } => {
            webhook::run(command, &cfg, cli.format).await?;
        }
    }

    Ok(())
}
