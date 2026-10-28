#[cfg(feature = "script")]
use done_mock::script::ScriptEngine;
#[cfg(feature = "postgres")]
use done_mock::store::postgres::PostgresMockStore;
use done_mock::{
    config::AppConfig,
    error::MockServerError,
    live_feed::LiveFeed,
    matching::MockIndex,
    mock_server::{create_app_router, MockServerState},
    model::Mock,
    responder::SequenceState,
    store::{memory::InMemoryMockStore, MockStore},
};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
#[cfg(feature = "script")]
use std::time::Duration;
use tracing::{debug, info, warn};
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

    let store: Arc<dyn MockStore> = {
        #[cfg(feature = "postgres")]
        if config.db_driver == "postgres" {
            info!("Using PostgreSQL store driver: {}", config.postgres_url);
            Arc::new(PostgresMockStore::new(&config.postgres_url).await?)
        } else {
            info!("Using in-memory store driver");
            Arc::new(InMemoryMockStore::new())
        }
        #[cfg(not(feature = "postgres"))]
        {
            info!("Using in-memory store driver (postgres feature disabled)");
            Arc::new(InMemoryMockStore::new())
        }
    };

    // Initialize the store (creates tables for PostgreSQL, no-op for in-memory)
    store.init().await?;

    // Seed mocks from directory if configured
    if let Some(ref mocks_dir) = config.mocks_dir {
        seed_mocks_from_directory(&*store, mocks_dir).await;
    }

    let sequence_state = SequenceState::new();
    #[cfg(feature = "script")]
    let script_engine = Some(ScriptEngine::new(Duration::from_millis(
        config.script_timeout_ms,
    )));
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
        #[cfg(feature = "script")]
        script_engine,
        live_feed,
    };

    let app_router = create_app_router(server_state);
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| MockServerError::DatabaseError(format!("Failed to bind address: {}", e)))?;

    info!("Mock server listening on http://{}", addr);

    // Graceful shutdown on SIGINT (Ctrl+C) or SIGTERM (container stop)
    let shutdown_signal = async {
        // Wait for Ctrl+C (SIGINT)
        let _ = tokio::signal::ctrl_c().await;
    };

    axum::serve(listener, app_router)
        .with_graceful_shutdown(shutdown_signal)
        .await
        .map_err(|e| MockServerError::DatabaseError(format!("Server error: {}", e)))?;

    info!("Server stopped");
    Ok(())
}

/// Loads all `.json` files from the given directory and creates them as mocks.
///
/// Skips files that fail to parse or validate, logging a warning for each.
async fn seed_mocks_from_directory(store: &dyn MockStore, dir: &str) {
    let path = Path::new(dir);
    if !path.exists() {
        warn!(
            mocks_dir = dir,
            "Mocks directory does not exist, skipping seed loading"
        );
        return;
    }

    if !path.is_dir() {
        warn!(
            mocks_dir = dir,
            "Mocks path is not a directory, skipping seed loading"
        );
        return;
    }

    let mut loaded = 0;
    let mut skipped = 0;

    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) => {
            warn!(error = %e, "Failed to read mocks directory, skipping seed loading");
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                warn!(error = %e, "Failed to read directory entry, skipping");
                continue;
            }
        };

        let file_path = entry.path();
        if file_path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }

        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let content = match std::fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                warn!(file = file_name, error = %e, "Failed to read mock file, skipping");
                skipped += 1;
                continue;
            }
        };

        let mock: Mock = match serde_json::from_str(&content) {
            Ok(m) => m,
            Err(e) => {
                warn!(file = file_name, error = %e, "Failed to parse mock JSON, skipping");
                skipped += 1;
                continue;
            }
        };

        // Validate the mock
        if let Err(e) = mock.validate() {
            warn!(file = file_name, error = %e, "Mock validation failed, skipping");
            skipped += 1;
            continue;
        }

        debug!(file = file_name, mock_id = %mock.id, "Creating mock from seed file");
        match store.create(mock).await {
            Ok(_) => loaded += 1,
            Err(e) => {
                warn!(file = file_name, error = %e, "Failed to create mock, skipping");
                skipped += 1;
            }
        }
    }

    info!(
        loaded = loaded,
        skipped = skipped,
        "Seed loading completed from {}",
        dir
    );
}
