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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();
        assert_eq!(config.port, 8000);
        assert_eq!(config.db_driver, "memory");
        assert_eq!(
            config.postgres_url,
            "postgres://mock:mock@localhost:5432/mocks"
        );
        assert_eq!(config.script_timeout_ms, 2000);
    }

    #[test]
    fn test_app_config_clone() {
        let config = AppConfig::default();
        let cloned = config.clone();
        assert_eq!(cloned.port, config.port);
        assert_eq!(cloned.db_driver, config.db_driver);
        assert_eq!(cloned.postgres_url, config.postgres_url);
        assert_eq!(cloned.script_timeout_ms, config.script_timeout_ms);
    }

    #[test]
    fn test_app_config_serialization() {
        let config = AppConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"port\":8000"));
        assert!(json.contains("\"db_driver\":\"memory\""));
    }

    #[test]
    fn test_app_config_deserialization() {
        let json = r#"{"port": 3000, "db_driver": "postgres", "postgres_url": "postgres://u:p@h/d", "script_timeout_ms": 1000}"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.port, 3000);
        assert_eq!(config.db_driver, "postgres");
        assert_eq!(config.script_timeout_ms, 1000);
    }

    #[test]
    fn test_app_config_debug() {
        let config = AppConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("AppConfig"));
        assert!(debug_str.contains("8000"));
    }
}
