use super::MockStore;
use crate::model::{
    default_iso_now, Condition, Mock, MockResponse, MockType, PartialMock, SequenceMode,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct MockConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<MockResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Vec<Condition>>,
    #[serde(
        default,
        rename = "elseResponse",
        skip_serializing_if = "Option::is_none"
    )]
    pub else_response: Option<MockResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<Vec<MockResponse>>,
    #[serde(
        default,
        rename = "sequenceMode",
        skip_serializing_if = "Option::is_none"
    )]
    pub sequence_mode: Option<SequenceMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    #[serde(
        default,
        rename = "scriptTimeoutMs",
        skip_serializing_if = "Option::is_none"
    )]
    pub script_timeout_ms: Option<u64>,
    #[serde(
        default,
        rename = "restIdField",
        skip_serializing_if = "Option::is_none"
    )]
    pub rest_id_field: Option<String>,
    #[serde(
        default,
        rename = "restInitialState",
        skip_serializing_if = "Option::is_none"
    )]
    pub rest_initial_state: Option<Vec<serde_json::Value>>,
}

#[derive(Clone)]
pub struct PostgresMockStore {
    pool: PgPool,
}

impl PostgresMockStore {
    pub async fn new(postgres_url: &str) -> Result<Self> {
        let max_connections: u32 = std::env::var("POOL_MAX_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);

        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(postgres_url)
            .await?;

        let store = Self { pool };
        store.init().await?;
        Ok(store)
    }

    fn row_to_mock(row: &sqlx::postgres::PgRow) -> Result<Mock> {
        let id: String = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let method: String = row.try_get("method")?;
        let path: String = row.try_get("path")?;
        let type_str: String = row.try_get("type")?;
        let priority: i32 = row.try_get("priority")?;
        let config_val: serde_json::Value = row.try_get("config")?;
        let state_val: Option<serde_json::Value> = row.try_get("state")?;
        let created_at: String = row
            .try_get("createdAt")
            .or_else(|_| row.try_get("createdat"))?;
        let updated_at: String = row
            .try_get("updatedAt")
            .or_else(|_| row.try_get("updatedat"))?;

        let mock_type: MockType = match type_str.to_lowercase().as_str() {
            "static" => MockType::Static,
            "conditional" => MockType::Conditional,
            "sequence" => MockType::Sequence,
            "script" => MockType::Script,
            "rest" => MockType::Rest,
            _ => serde_json::from_str(&format!("\"{}\"", type_str)).unwrap_or(MockType::Static),
        };

        let config: MockConfig = serde_json::from_value(config_val).unwrap_or_default();
        let state: Option<Vec<serde_json::Value>> =
            state_val.and_then(|v| serde_json::from_value(v).ok());

        Ok(Mock {
            id,
            name,
            method,
            path,
            mock_type,
            priority,
            response: config.response,
            conditions: config.conditions,
            else_response: config.else_response,
            sequence: config.sequence,
            sequence_mode: config.sequence_mode,
            script: config.script,
            script_timeout_ms: config.script_timeout_ms,
            rest_id_field: config.rest_id_field,
            rest_initial_state: config.rest_initial_state,
            state,
            created_at,
            updated_at,
        })
    }
}

#[async_trait]
impl MockStore for PostgresMockStore {
    async fn init(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS mocks (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                method TEXT NOT NULL,
                path TEXT NOT NULL,
                type TEXT NOT NULL,
                priority INTEGER NOT NULL,
                config JSONB NOT NULL,
                state JSONB,
                "createdAt" TEXT NOT NULL,
                "updatedAt" TEXT NOT NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn list(&self) -> Result<Vec<Mock>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, method, path, type, priority, config, state, "createdAt", "updatedAt"
            FROM mocks
            ORDER BY priority DESC, "createdAt" DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut mocks = Vec::with_capacity(rows.len());
        for row in rows {
            mocks.push(Self::row_to_mock(&row)?);
        }
        Ok(mocks)
    }

    async fn get(&self, id: &str) -> Result<Option<Mock>> {
        let maybe_row = sqlx::query(
            r#"
            SELECT id, name, method, path, type, priority, config, state, "createdAt", "updatedAt"
            FROM mocks
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match maybe_row {
            Some(row) => Ok(Some(Self::row_to_mock(&row)?)),
            None => Ok(None),
        }
    }

    async fn create(&self, mut mock: Mock) -> Result<Mock> {
        if mock.id.trim().is_empty() {
            mock.id = Uuid::new_v4().to_string();
        }
        let now = default_iso_now();
        if mock.created_at.trim().is_empty() {
            mock.created_at = now.clone();
        }
        if mock.updated_at.trim().is_empty() {
            mock.updated_at = now;
        }

        if mock.mock_type == MockType::Rest && mock.state.is_none() {
            mock.state = mock.rest_initial_state.clone();
        }

        let config = MockConfig {
            response: mock.response.clone(),
            conditions: mock.conditions.clone(),
            else_response: mock.else_response.clone(),
            sequence: mock.sequence.clone(),
            sequence_mode: mock.sequence_mode.clone(),
            script: mock.script.clone(),
            script_timeout_ms: mock.script_timeout_ms,
            rest_id_field: mock.rest_id_field.clone(),
            rest_initial_state: mock.rest_initial_state.clone(),
        };

        let config_val = serde_json::to_value(&config)?;
        let state_val = serde_json::to_value(&mock.state)?;
        let type_str = mock.mock_type.to_string();

        sqlx::query(
            r#"
            INSERT INTO mocks (id, name, method, path, type, priority, config, state, "createdAt", "updatedAt")
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(&mock.id)
        .bind(&mock.name)
        .bind(&mock.method)
        .bind(&mock.path)
        .bind(&type_str)
        .bind(mock.priority)
        .bind(config_val)
        .bind(state_val)
        .bind(&mock.created_at)
        .bind(&mock.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(mock)
    }

    async fn update(&self, id: &str, partial: PartialMock) -> Result<Mock> {
        let mut existing = self
            .get(id)
            .await?
            .ok_or_else(|| anyhow!("Mock with id '{}' not found", id))?;

        existing.apply_partial(partial);

        let config = MockConfig {
            response: existing.response.clone(),
            conditions: existing.conditions.clone(),
            else_response: existing.else_response.clone(),
            sequence: existing.sequence.clone(),
            sequence_mode: existing.sequence_mode.clone(),
            script: existing.script.clone(),
            script_timeout_ms: existing.script_timeout_ms,
            rest_id_field: existing.rest_id_field.clone(),
            rest_initial_state: existing.rest_initial_state.clone(),
        };

        let config_val = serde_json::to_value(&config)?;
        let state_val = serde_json::to_value(&existing.state)?;
        let type_str = existing.mock_type.to_string();

        sqlx::query(
            r#"
            UPDATE mocks
            SET name = $1, method = $2, path = $3, type = $4, priority = $5, config = $6, state = $7, "updatedAt" = $8
            WHERE id = $9
            "#,
        )
        .bind(&existing.name)
        .bind(&existing.method)
        .bind(&existing.path)
        .bind(&type_str)
        .bind(existing.priority)
        .bind(config_val)
        .bind(state_val)
        .bind(&existing.updated_at)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(existing)
    }

    async fn update_state(&self, id: &str, state: serde_json::Value) -> Result<()> {
        let updated_at = default_iso_now();
        let res = sqlx::query(
            r#"
            UPDATE mocks
            SET state = $1, "updatedAt" = $2
            WHERE id = $3
            "#,
        )
        .bind(state)
        .bind(updated_at)
        .bind(id)
        .execute(&self.pool)
        .await?;

        if res.rows_affected() == 0 {
            return Err(anyhow!("Mock with id '{}' not found", id));
        }

        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let res = sqlx::query(
            r#"
            DELETE FROM mocks
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if res.rows_affected() == 0 {
            return Err(anyhow!("Mock with id '{}' not found", id));
        }

        Ok(())
    }
}
