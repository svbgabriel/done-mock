use super::MockStore;
use crate::error::MockServerError;
use crate::model::{default_iso_now, Mock, MockType, PartialMock};
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
    async fn init(&self) -> super::StoreResult<()> {
        Ok(())
    }

    async fn list(&self) -> super::StoreResult<Vec<Mock>> {
        let guard = self.mocks.read().unwrap();
        let mut list: Vec<Mock> = guard.values().cloned().collect();
        list.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| b.created_at.cmp(&a.created_at))
        });
        Ok(list)
    }

    async fn get(&self, id: &str) -> super::StoreResult<Option<Mock>> {
        let guard = self.mocks.read().unwrap();
        Ok(guard.get(id).cloned())
    }

    async fn create(&self, mut mock: Mock) -> super::StoreResult<Mock> {
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

    async fn update(&self, id: &str, partial: PartialMock) -> super::StoreResult<Mock> {
        let mut guard = self.mocks.write().unwrap();
        let existing = guard
            .get_mut(id)
            .ok_or_else(|| MockServerError::NotFound(id.to_string()))?;

        existing.apply_partial(partial);

        Ok(existing.clone())
    }

    async fn update_state(&self, id: &str, state: serde_json::Value) -> super::StoreResult<()> {
        let mut guard = self.mocks.write().unwrap();
        let existing = guard
            .get_mut(id)
            .ok_or_else(|| MockServerError::NotFound(id.to_string()))?;

        let parsed: Vec<serde_json::Value> = serde_json::from_value(state).unwrap_or_default();
        existing.state = Some(parsed);
        existing.updated_at = default_iso_now();

        Ok(())
    }

    async fn delete(&self, id: &str) -> super::StoreResult<()> {
        let mut guard = self.mocks.write().unwrap();
        guard
            .remove(id)
            .ok_or_else(|| MockServerError::NotFound(id.to_string()))?;
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

        let mock_a = Mock::builder()
            .id("m-1")
            .name("Mock A")
            .method("GET")
            .path("/a")
            .priority(10)
            .response(MockResponse::default())
            .created_at("2026-01-01T00:00:00.000Z")
            .updated_at("2026-01-01T00:00:00.000Z")
            .build();

        let mock_b = Mock::builder()
            .id("m-2")
            .name("Mock B")
            .method("GET")
            .path("/b")
            .priority(20)
            .response(MockResponse::default())
            .created_at("2026-01-01T00:00:00.000Z")
            .updated_at("2026-01-01T00:00:00.000Z")
            .build();

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
        let mock = Mock::builder()
            .id("m-1")
            .name("Test")
            .method("GET")
            .path("/test")
            .response(MockResponse::default())
            .created_at("2026-01-01T00:00:00.000Z")
            .updated_at("2026-01-01T00:00:00.000Z")
            .build();
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
        let mock = Mock::builder()
            .id("")
            .name("Auto ID")
            .method("GET")
            .path("/test")
            .response(MockResponse::default())
            .created_at("")
            .updated_at("")
            .build();
        let created = store.create(mock).await.unwrap();
        assert!(!created.id.is_empty());
        assert!(!created.created_at.is_empty());
        assert!(!created.updated_at.is_empty());
    }

    #[tokio::test]
    async fn test_in_memory_store_create_preserves_id() {
        let store = InMemoryMockStore::new();
        let mock = Mock::builder()
            .id("custom-id")
            .name("Custom ID")
            .method("GET")
            .path("/test")
            .response(MockResponse::default())
            .created_at("")
            .updated_at("")
            .build();
        let created = store.create(mock).await.unwrap();
        assert_eq!(created.id, "custom-id");
    }

    #[tokio::test]
    async fn test_in_memory_store_create_rest_initializes_state() {
        let store = InMemoryMockStore::new();
        let mock = Mock::builder()
            .id("rest-1")
            .name("REST Mock")
            .method("GET")
            .path("/items")
            .mock_type(MockType::Rest)
            .rest_id_field("id")
            .rest_initial_state(vec![
                serde_json::json!({"id": "1", "name": "Item 1"}),
                serde_json::json!({"id": "2", "name": "Item 2"}),
            ])
            .created_at("")
            .updated_at("")
            .build();
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
        let mock = Mock::builder()
            .id("m-1")
            .name("Original")
            .method("GET")
            .path("/test")
            .response(MockResponse::default())
            .created_at("2026-01-01T00:00:00.000Z")
            .updated_at("2026-01-01T00:00:00.000Z")
            .build();
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
        let mock = Mock::builder()
            .id("m-1")
            .name("Test")
            .method("GET")
            .path("/old")
            .response(MockResponse::default())
            .created_at("2026-01-01T00:00:00.000Z")
            .updated_at("2026-01-01T00:00:00.000Z")
            .build();
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
        let mock = Mock::builder()
            .id("m-1")
            .name("REST")
            .method("GET")
            .path("/items")
            .mock_type(MockType::Rest)
            .state(vec![serde_json::json!({"id": "1"})])
            .created_at("2026-01-01T00:00:00.000Z")
            .updated_at("2026-01-01T00:00:00.000Z")
            .build();
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
        let mock = Mock::builder()
            .id("m-1")
            .name("Test")
            .method("GET")
            .path("/test")
            .response(MockResponse::default())
            .created_at("2026-01-01T00:00:00.000Z")
            .updated_at("2026-01-01T00:00:00.000Z")
            .build();
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
        let mock = Mock::builder()
            .id("m-1")
            .name("Original")
            .method("GET")
            .path("/old")
            .response(MockResponse::default())
            .created_at("2026-01-01T00:00:00.000Z")
            .updated_at("2026-01-01T00:00:00.000Z")
            .build();
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
        let mock_older = Mock::builder()
            .id("older")
            .name("Older")
            .method("GET")
            .path("/a")
            .priority(5)
            .response(MockResponse::default())
            .created_at("2026-01-01T00:00:00.000Z")
            .updated_at("2026-01-01T00:00:00.000Z")
            .build();

        let mock_newer = Mock::builder()
            .id("newer")
            .name("Newer")
            .method("GET")
            .path("/b")
            .priority(5)
            .response(MockResponse::default())
            .created_at("2026-02-01T00:00:00.000Z")
            .updated_at("2026-02-01T00:00:00.000Z")
            .build();

        store.create(mock_older).await.unwrap();
        store.create(mock_newer).await.unwrap();

        let list = store.list().await.unwrap();
        assert_eq!(list.len(), 2);
        // Same priority, newer created_at first
        assert_eq!(list[0].id, "newer");
        assert_eq!(list[1].id, "older");
    }
}
