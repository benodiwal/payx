pub mod api;
pub mod config;
pub mod domain;
pub mod error;
pub mod telemetry;
pub mod workers;

use anyhow::Result;
use axum::Router;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::sync::Arc;

use crate::api::routes;
use crate::config::Config;
use crate::workers::webhook_processor::WebhookProcessor;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Config,
    pub http_client: reqwest::Client,
}

pub struct App {
    state: Arc<AppState>,
    _webhook_processor: WebhookProcessor,
}

impl App {
    pub fn db(&self) -> &PgPool {
        &self.state.db
    }

    pub async fn new(config: Config) -> Result<Self> {
        let db = PgPoolOptions::new()
            .max_connections(config.db_max_connections)
            .connect(&config.database_url)
            .await?;

        sqlx::migrate!("./migrations").run(&db).await?;

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let state = Arc::new(AppState {
            db: db.clone(),
            config: config.clone(),
            http_client: http_client.clone(),
        });

        let mut webhook_processor = WebhookProcessor::new(db, http_client);
        webhook_processor.start();

        Ok(Self {
            state,
            _webhook_processor: webhook_processor,
        })
    }

    pub fn router(&self) -> Router {
        routes::build(self.state.clone())
    }
}
