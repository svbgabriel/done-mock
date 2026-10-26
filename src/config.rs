use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub port: u16,
    pub db_driver: String,
    pub postgres_url: String,
    pub script_timeout_ms: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            port: 8000,
            db_driver: "memory".to_string(),
            postgres_url: "postgres://mock:mock@localhost:5432/mocks".to_string(),
            script_timeout_ms: 2000,
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        let _ = dotenvy::dotenv();

        let port = env::var("PORT")
            .or_else(|_| env::var("MOCK_PORT"))
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(8000);

        let db_driver = env::var("DB_DRIVER")
            .unwrap_or_else(|_| "memory".to_string())
            .to_lowercase();

        let postgres_url = env::var("POSTGRES_URL")
            .or_else(|_| env::var("DATABASE_URL"))
            .unwrap_or_else(|_| "postgres://mock:mock@localhost:5432/mocks".to_string());

        let script_timeout_ms = env::var("SCRIPT_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2000);

        Self {
            port,
            db_driver,
            postgres_url,
            script_timeout_ms,
        }
    }
}
