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

    #[tokio::test]
    async fn test_in_memory_store_init() {
        let store = InMemoryMockStore::new();
        store.init().await.unwrap();
    }

    #[tokio::test]
    async fn test_in_memory_store_list_empty() {
        let store = InMemoryMockStore::new();
        let list = store.list().await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_in_memory_store_get_existing() {
        let store = InMemoryMockStore::new();
        let mock = Mock {
            id: "m-1".to_string(),
            name: "Test".to_string(),
            method: "GET".to_string(),
            path: "/test".to_string(),
            mock_type: MockType::Static,
            priority: 0,
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
        store.create(mock).await.unwrap();
        let found = store.get("m-1").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test");
    }

    #[tokio::test]
    async fn test_in_memory_store_get_nonexistent() {
        let store = InMemoryMockStore::new();
        let found = store.get("nonexistent").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_in_memory_store_create_generates_id() {
        let store = InMemoryMockStore::new();
        let mock = Mock {
            id: "".to_string(),
            name: "Auto ID".to_string(),
            method: "GET".to_string(),
            path: "/test".to_string(),
            mock_type: MockType::Static,
            priority: 0,
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
            created_at: "".to_string(),
            updated_at: "".to_string(),
        };
        let created = store.create(mock).await.unwrap();
        assert!(!created.id.is_empty());
        assert!(!created.created_at.is_empty());
        assert!(!created.updated_at.is_empty());
    }

    #[tokio::test]
    async fn test_in_memory_store_create_preserves_id() {
        let store = InMemoryMockStore::new();
        let mock = Mock {
            id: "custom-id".to_string(),
            name: "Custom ID".to_string(),
            method: "GET".to_string(),
            path: "/test".to_string(),
            mock_type: MockType::Static,
            priority: 0,
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
            created_at: "".to_string(),
            updated_at: "".to_string(),
        };
        let created = store.create(mock).await.unwrap();
        assert_eq!(created.id, "custom-id");
    }

    #[tokio::test]
    async fn test_in_memory_store_create_rest_initializes_state() {
        let store = InMemoryMockStore::new();
        let mock = Mock {
            id: "rest-1".to_string(),
            name: "REST Mock".to_string(),
            method: "GET".to_string(),
            path: "/items".to_string(),
            mock_type: MockType::Rest,
            priority: 0,
            response: None,
            conditions: None,
            else_response: None,
            sequence: None,
            sequence_mode: None,
            script: None,
            script_timeout_ms: None,
            rest_id_field: Some("id".to_string()),
            rest_initial_state: Some(vec![
                serde_json::json!({"id": "1", "name": "Item 1"}),
                serde_json::json!({"id": "2", "name": "Item 2"}),
            ]),
            state: None,
            created_at: "".to_string(),
            updated_at: "".to_string(),
        };
        let created = store.create(mock).await.unwrap();
        assert!(created.state.is_some());
        let state = created.state.unwrap();
        assert_eq!(state.len(), 2);
    }

    #[tokio::test]
    async fn test_in_memory_store_update_not_found() {
        let store = InMemoryMockStore::new();
        let result = store
            .update(
                "nonexistent",
                PartialMock {
                    name: Some("Updated".to_string()),
                    ..Default::default()
                },
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_in_memory_store_update_name() {
        let store = InMemoryMockStore::new();
        let mock = Mock {
            id: "m-1".to_string(),
            name: "Original".to_string(),
            method: "GET".to_string(),
            path: "/test".to_string(),
            mock_type: MockType::Static,
            priority: 0,
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
        store.create(mock).await.unwrap();

        let updated = store
            .update(
                "m-1",
                PartialMock {
                    name: Some("Renamed".to_string()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.name, "Renamed");
    }

    #[tokio::test]
    async fn test_in_memory_store_update_method_and_path() {
        let store = InMemoryMockStore::new();
        let mock = Mock {
            id: "m-1".to_string(),
            name: "Test".to_string(),
            method: "GET".to_string(),
            path: "/old".to_string(),
            mock_type: MockType::Static,
            priority: 0,
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
        store.create(mock).await.unwrap();

        let updated = store
            .update(
                "m-1",
                PartialMock {
                    method: Some("POST".to_string()),
                    path: Some("/new".to_string()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.method, "POST");
        assert_eq!(updated.path, "/new");
    }

    #[tokio::test]
    async fn test_in_memory_store_delete_not_found() {
        let store = InMemoryMockStore::new();
        let result = store.delete("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_in_memory_store_update_state() {
        let store = InMemoryMockStore::new();
        let mock = Mock {
            id: "m-1".to_string(),
            name: "REST".to_string(),
            method: "GET".to_string(),
            path: "/items".to_string(),
            mock_type: MockType::Rest,
            priority: 0,
            response: None,
            conditions: None,
            else_response: None,
            sequence: None,
            sequence_mode: None,
            script: None,
            script_timeout_ms: None,
            rest_id_field: None,
            rest_initial_state: None,
            state: Some(vec![serde_json::json!({"id": "1"})]),
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
            updated_at: "2026-01-01T00:00:00.000Z".to_string(),
        };
        store.create(mock).await.unwrap();

        let new_state = serde_json::json!([
            {"id": "1"},
            {"id": "2"}
        ]);
        store.update_state("m-1", new_state).await.unwrap();

        let updated = store.get("m-1").await.unwrap().unwrap();
        assert!(updated.state.is_some());
        assert_eq!(updated.state.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_in_memory_store_update_state_not_found() {
        let store = InMemoryMockStore::new();
        let result = store
            .update_state("nonexistent", serde_json::json!([]))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_in_memory_store_clone() {
        let store = InMemoryMockStore::new();
        let mock = Mock {
            id: "m-1".to_string(),
            name: "Test".to_string(),
            method: "GET".to_string(),
            path: "/test".to_string(),
            mock_type: MockType::Static,
            priority: 0,
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
        store.create(mock).await.unwrap();

        let store2 = store.clone();
        let list = store2.list().await.unwrap();
        assert_eq!(list.len(), 1);
    }

    #[tokio::test]
    async fn test_in_memory_store_default() {
        let store = InMemoryMockStore::default();
        let list = store.list().await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_in_memory_store_update_all_fields() {
        let store = InMemoryMockStore::new();
        let mock = Mock {
            id: "m-1".to_string(),
            name: "Original".to_string(),
            method: "GET".to_string(),
            path: "/old".to_string(),
            mock_type: MockType::Static,
            priority: 0,
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
        store.create(mock).await.unwrap();

        let updated = store
            .update(
                "m-1",
                PartialMock {
                    name: Some("Updated".to_string()),
                    method: Some("POST".to_string()),
                    path: Some("/new".to_string()),
                    mock_type: Some(MockType::Conditional),
                    priority: Some(10),
                    response: Some(MockResponse {
                        status: 201,
                        headers: None,
                        body: Some("created".to_string()),
                        content_type: None,
                    }),
                    conditions: None,
                    else_response: None,
                    sequence: None,
                    sequence_mode: None,
                    script: None,
                    script_timeout_ms: None,
                    rest_id_field: None,
                    rest_initial_state: None,
                    state: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.name, "Updated");
        assert_eq!(updated.method, "POST");
        assert_eq!(updated.path, "/new");
        assert_eq!(updated.mock_type, MockType::Conditional);
        assert_eq!(updated.priority, 10);
        assert_eq!(updated.response.as_ref().unwrap().status, 201);
    }

    #[tokio::test]
    async fn test_in_memory_store_list_sorting_by_created_at() {
        let store = InMemoryMockStore::new();

        // Same priority, different created_at
        let mock_older = Mock {
            id: "older".to_string(),
            name: "Older".to_string(),
            method: "GET".to_string(),
            path: "/a".to_string(),
            mock_type: MockType::Static,
            priority: 5,
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

        let mock_newer = Mock {
            id: "newer".to_string(),
            name: "Newer".to_string(),
            method: "GET".to_string(),
            path: "/b".to_string(),
            mock_type: MockType::Static,
            priority: 5,
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
            created_at: "2026-02-01T00:00:00.000Z".to_string(),
            updated_at: "2026-02-01T00:00:00.000Z".to_string(),
        };

        store.create(mock_older).await.unwrap();
        store.create(mock_newer).await.unwrap();

        let list = store.list().await.unwrap();
        assert_eq!(list.len(), 2);
        // Same priority, newer created_at first
        assert_eq!(list[0].id, "newer");
        assert_eq!(list[1].id, "older");
    }
}
