pub mod memory;
pub mod postgres;

use crate::model::{Mock, PartialMock};
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
#[allow(
    clippy::double_must_use,
    reason = "async_trait macro duplicates must_use in async methods"
)]
pub trait MockStore: Send + Sync {
    async fn init(&self) -> Result<()>;
    async fn list(&self) -> Result<Vec<Mock>>;
    async fn get(&self, id: &str) -> Result<Option<Mock>>;
    async fn create(&self, mock: Mock) -> Result<Mock>;
    async fn update(&self, id: &str, mock: PartialMock) -> Result<Mock>;
    async fn update_state(&self, id: &str, state: serde_json::Value) -> Result<()>;
    async fn delete(&self, id: &str) -> Result<()>;
}
