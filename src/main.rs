use mock_server::{
    config::AppConfig,
    live_feed::LiveFeed,
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
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mock_server=debug,tower_http=debug".into()),
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

    let server_state = MockServerState {
        store,
        sequence_state,
        script_engine: Some(script_engine),
        live_feed,
    };

    let app_router = create_app_router(server_state);
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("Mock server listening on http://{}", addr);
    axum::serve(listener, app_router).await?;

    Ok(())
}
