use super::MockStore;
use crate::model::{default_iso_now, Mock, MockType, PartialMock};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct InMemoryMockStore {
    mocks: Arc<RwLock<HashMap<String, Mock>>>,
}

impl InMemoryMockStore {
    pub fn new() -> Self {
        Self {
            mocks: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl MockStore for InMemoryMockStore {
    async fn init(&self) -> Result<()> {
        Ok(())
    }

    async fn list(&self) -> Result<Vec<Mock>> {
        let guard = self.mocks.read().unwrap();
        let mut list: Vec<Mock> = guard.values().cloned().collect();
        list.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| b.created_at.cmp(&a.created_at))
        });
        Ok(list)
    }

    async fn get(&self, id: &str) -> Result<Option<Mock>> {
        let guard = self.mocks.read().unwrap();
        Ok(guard.get(id).cloned())
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

        let mut guard = self.mocks.write().unwrap();
        guard.insert(mock.id.clone(), mock.clone());
        Ok(mock)
    }

    async fn update(&self, id: &str, partial: PartialMock) -> Result<Mock> {
        let mut guard = self.mocks.write().unwrap();
        let existing = guard
            .get_mut(id)
            .ok_or_else(|| anyhow!("Mock with id '{}' not found", id))?;

        if let Some(name) = partial.name {
            existing.name = name;
        }
        if let Some(method) = partial.method {
            existing.method = method;
        }
        if let Some(path) = partial.path {
            existing.path = path;
        }
        if let Some(mock_type) = partial.mock_type {
            existing.mock_type = mock_type;
        }
        if let Some(priority) = partial.priority {
            existing.priority = priority;
        }
        if partial.response.is_some() {
            existing.response = partial.response;
        }
        if partial.conditions.is_some() {
            existing.conditions = partial.conditions;
        }
        if partial.else_response.is_some() {
            existing.else_response = partial.else_response;
        }
        if partial.sequence.is_some() {
            existing.sequence = partial.sequence;
        }
        if partial.sequence_mode.is_some() {
            existing.sequence_mode = partial.sequence_mode;
        }
        if partial.script.is_some() {
            existing.script = partial.script;
        }
        if partial.script_timeout_ms.is_some() {
            existing.script_timeout_ms = partial.script_timeout_ms;
        }
        if partial.rest_id_field.is_some() {
            existing.rest_id_field = partial.rest_id_field;
        }
        if partial.rest_initial_state.is_some() {
            existing.rest_initial_state = partial.rest_initial_state;
        }
        if partial.state.is_some() {
            existing.state = partial.state;
        }

        existing.updated_at = default_iso_now();

        Ok(existing.clone())
    }

    async fn update_state(&self, id: &str, state: serde_json::Value) -> Result<()> {
        let mut guard = self.mocks.write().unwrap();
        let existing = guard
            .get_mut(id)
            .ok_or_else(|| anyhow!("Mock with id '{}' not found", id))?;

        let parsed: Vec<serde_json::Value> = serde_json::from_value(state).unwrap_or_default();
        existing.state = Some(parsed);
        existing.updated_at = default_iso_now();

        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let mut guard = self.mocks.write().unwrap();
        guard
            .remove(id)
            .ok_or_else(|| anyhow!("Mock with id '{}' not found", id))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::MockResponse;

    #[tokio::test]
    async fn test_in_memory_store_crud_and_sorting() {
        let store = InMemoryMockStore::new();

        let mock_a = Mock {
            id: "m-1".to_string(),
            name: "Mock A".to_string(),
            method: "GET".to_string(),
            path: "/a".to_string(),
            mock_type: MockType::Static,
            priority: 10,
            response: Some(MockResponse::default()),
            conditions: None,
            else_response: None,
            sequence: None,
            sequence_mode: None,
            script: None,
            script_timeout_ms: None,
            rest_id_field: None,
            rest_initial_state: None,
            state: None,
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
            updated_at: "2026-01-01T00:00:00.000Z".to_string(),
        };

        let mock_b = Mock {
            id: "m-2".to_string(),
            name: "Mock B".to_string(),
            method: "GET".to_string(),
            path: "/b".to_string(),
            mock_type: MockType::Static,
            priority: 20,
            response: Some(MockResponse::default()),
            conditions: None,
            else_response: None,
            sequence: None,
            sequence_mode: None,
            script: None,
            script_timeout_ms: None,
            rest_id_field: None,
            rest_initial_state: None,
            state: None,
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
            updated_at: "2026-01-01T00:00:00.000Z".to_string(),
        };

        store.create(mock_a).await.unwrap();
        store.create(mock_b).await.unwrap();

        let list = store.list().await.unwrap();
        assert_eq!(list.len(), 2);
        // priority 20 before priority 10
        assert_eq!(list[0].id, "m-2");
        assert_eq!(list[1].id, "m-1");

        // Update
        let updated = store
            .update(
                "m-1",
                PartialMock {
                    priority: Some(30),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.priority, 30);

        let list2 = store.list().await.unwrap();
        assert_eq!(list2[0].id, "m-1"); // priority 30 now first

        // Delete
        store.delete("m-2").await.unwrap();
        let list3 = store.list().await.unwrap();
        assert_eq!(list3.len(), 1);
        assert_eq!(list3[0].id, "m-1");
    }
}
