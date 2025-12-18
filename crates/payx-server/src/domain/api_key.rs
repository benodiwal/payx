use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApiKey {
    pub id: Uuid,
    pub business_id: Uuid,
    pub key_hash: String,
    pub key_prefix: String,
    pub name: Option<String>,
    pub rate_limit_per_minute: i32,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct GeneratedApiKey {
    pub id: Uuid,
    pub key: String,
    pub prefix: String,
}

impl ApiKey {
    pub fn generate(business_id: Uuid) -> (Self, GeneratedApiKey) {
        let mut key_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key_bytes);

        let key = format!("payx_{}", URL_SAFE_NO_PAD.encode(key_bytes));
        let prefix = key[..12].to_string();

        let salt = SaltString::generate(&mut rand::thread_rng());
        let key_hash = Argon2::default()
            .hash_password(key.as_bytes(), &salt)
            .expect("failed to hash")
            .to_string();

        let id = Uuid::new_v4();

        let api_key = Self {
            id,
            business_id,
            key_hash,
            key_prefix: prefix.clone(),
            name: None,
            rate_limit_per_minute: 100,
            created_at: Utc::now(),
            expires_at: None,
            revoked_at: None,
            last_used_at: None,
        };

        let generated = GeneratedApiKey { id, key, prefix };

        (api_key, generated)
    }

    pub fn verify(&self, key: &str) -> bool {
        let parsed = match PasswordHash::new(&self.key_hash) {
            Ok(h) => h,
            Err(_) => return false,
        };
        Argon2::default()
            .verify_password(key.as_bytes(), &parsed)
            .is_ok()
    }

    pub fn is_valid(&self) -> bool {
        if self.revoked_at.is_some() {
            return false;
        }
        if let Some(expires) = self.expires_at {
            if expires < Utc::now() {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: Option<String>,
}
