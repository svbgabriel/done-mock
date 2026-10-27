use done_mock::{
    config::AppConfig,
    error::MockServerError,
    live_feed::LiveFeed,
    matching::MockIndex,
    mock_server::{create_app_router, MockServerState},
    responder::SequenceState,
    script::ScriptEngine,
    store::{memory::InMemoryMockStore, postgres::PostgresMockStore, MockStore},
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), MockServerError> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "done_mock=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::load();
    info!("Starting mock server with config: {:?}", config);

    let store: Arc<dyn MockStore> = if config.db_driver == "postgres" {
        info!("Using PostgreSQL store driver: {}", config.postgres_url);
        Arc::new(PostgresMockStore::new(&config.postgres_url).await?)
    } else {
        info!("Using in-memory store driver");
        Arc::new(InMemoryMockStore::new())
    };

    let sequence_state = SequenceState::new();
    let script_engine = ScriptEngine::new(Duration::from_millis(config.script_timeout_ms));
    let live_feed = LiveFeed::new();

    // Initialize mock cache and index from store
    let initial_mocks = store.list().await.unwrap_or_default();
    let mock_cache = Arc::new(tokio::sync::RwLock::new(initial_mocks.clone()));
    let mock_index = Arc::new(tokio::sync::RwLock::new(MockIndex::build(&initial_mocks)));

    let server_state = MockServerState {
        store,
        mock_cache,
        mock_index,
        sequence_state,
        script_engine: Some(script_engine),
        live_feed,
    };

    let app_router = create_app_router(server_state);
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| MockServerError::DatabaseError(format!("Failed to bind address: {}", e)))?;

    info!("Mock server listening on http://{}", addr);
    axum::serve(listener, app_router)
        .await
        .map_err(|e| MockServerError::DatabaseError(format!("Server error: {}", e)))?;

    Ok(())
}
