use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{de::DeserializeOwned, Serialize};

use crate::config::Config;

pub struct ApiClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
}

impl ApiClient {
    pub fn new(config: &Config) -> Self {
        Self {
            client: Client::new(),
            base_url: config.server.trim_end_matches('/').to_string(),
            api_key: config.api_key.clone(),
        }
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.get(&url);

        if let Some(key) = &self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let resp = req.send().await.context("request failed")?;
        self.handle_response(resp).await
    }

    pub async fn post<T: DeserializeOwned, B: Serialize>(&self, path: &str, body: &B) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.post(&url).json(body);

        if let Some(key) = &self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let resp = req.send().await.context("request failed")?;
        self.handle_response(resp).await
    }

    pub async fn post_with_idempotency<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
        idempotency_key: Option<&str>,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.post(&url).json(body);

        if let Some(key) = &self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        if let Some(idem_key) = idempotency_key {
            req = req.header("Idempotency-Key", idem_key);
        }

        let resp = req.send().await.context("request failed")?;
        self.handle_response(resp).await
    }

    pub async fn post_no_auth<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .post(&url)
            .json(body)
            .send()
            .await
            .context("request failed")?;
        self.handle_response(resp).await
    }

    async fn handle_response<T: DeserializeOwned>(&self, resp: reqwest::Response) -> Result<T> {
        let status = resp.status();
        let body = resp.text().await.context("failed to read response")?;

        if !status.is_success() {
            if let Ok(err) = serde_json::from_str::<serde_json::Value>(&body) {
                if let Some(error) = err.get("error") {
                    let code = error
                        .get("code")
                        .and_then(|c| c.as_str())
                        .unwrap_or("unknown");
                    let message = error
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("unknown error");
                    bail!("{}: {}", code, message);
                }
            }
            bail!("request failed with status {}: {}", status, body);
        }

        serde_json::from_str(&body).context("failed to parse response")
    }
}
