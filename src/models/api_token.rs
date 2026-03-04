use chrono::{DateTime, Utc};
use rand::Rng;
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[allow(dead_code)]
pub struct ApiToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub access_token_hash: String,
    pub refresh_token_hash: String,
    pub device_name: Option<String>,
    pub device_id: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub refresh_expires_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[allow(dead_code)]
pub fn generate_token(prefix: &str) -> (String, String) {
    let mut rng = rand::rng();
    let mut secret_bytes = [0u8; 32];
    rng.fill_bytes(&mut secret_bytes);
    let token = format!("{}{}", prefix, hex::encode(secret_bytes));
    let hash = hex::encode(Sha256::digest(token.as_bytes()));
    (token, hash) // plaintext → client, hash → DB
}
