pub mod memory;
pub mod postgres;

use crate::error::MockServerError;
use crate::model::{Mock, PartialMock};
use async_trait::async_trait;

type StoreResult<T> = Result<T, MockServerError>;

#[async_trait]
#[allow(
    clippy::double_must_use,
    reason = "async_trait macro duplicates must_use in async methods"
)]
pub trait MockStore: Send + Sync {
    async fn init(&self) -> StoreResult<()>;
    async fn list(&self) -> StoreResult<Vec<Mock>>;
    async fn get(&self, id: &str) -> StoreResult<Option<Mock>>;
    async fn create(&self, mock: Mock) -> StoreResult<Mock>;
    async fn update(&self, id: &str, mock: PartialMock) -> StoreResult<Mock>;
    async fn update_state(&self, id: &str, state: serde_json::Value) -> StoreResult<()>;
    async fn delete(&self, id: &str) -> StoreResult<()>;
}
