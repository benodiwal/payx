use anyhow::{Context, Result};
use std::env;

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_address: String,
    pub db_max_connections: u32,
    pub otlp_endpoint: Option<String>,
    pub rate_limit_per_minute: i32,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: env::var("DATABASE_URL").context("DATABASE_URL required")?,
            bind_address: env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8080".into()),
            db_max_connections: env::var("DB_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "20".into())
                .parse()?,
            otlp_endpoint: env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok(),
            rate_limit_per_minute: env::var("RATE_LIMIT_PER_MINUTE")
                .unwrap_or_else(|_| "100".into())
                .parse()?,
        })
    }
}
