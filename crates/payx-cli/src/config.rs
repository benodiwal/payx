use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_server")]
    pub server: String,
    pub api_key: Option<String>,
}

fn default_server() -> String {
    "http://localhost:8080".into()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: default_server(),
            api_key: None,
        }
    }
}

pub fn config_path() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("could not find config directory")?
        .join("payx");
    Ok(dir.join("config.toml"))
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }
}
