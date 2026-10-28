use crate::error::MockServerError;
use crate::mock_server::MockServerState;
use crate::model::{Mock, MockType, PartialMock};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    routing::{get, post},
    Json, Router,
};
use futures_util::stream::{self, Stream, StreamExt};
use serde_json::json;
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;

pub fn create_admin_router() -> Router<MockServerState> {
    Router::new()
        .route("/__admin/health", get(health_check))
        .route("/__admin/mocks", get(list_mocks).post(create_mock))
        .route("/__admin/mocks/bulk", post(bulk_create_mocks))
        .route("/__admin/mocks/reset", post(reset_all_mocks))
        .route(
            "/__admin/mocks/{id}",
            get(get_mock).put(update_mock).delete(delete_mock),
        )
        .route("/__admin/mocks/{id}/reset", post(reset_mock))
        .route("/__admin/livefeed", get(livefeed_sse))
}

async fn health_check() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}

fn store_error_response(err: MockServerError) -> (StatusCode, Json<serde_json::Value>) {
    (
        err.status_code(),
        Json(json!({ "error": err.error_type(), "message": err.to_string() })),
    )
}

async fn list_mocks(State(state): State<MockServerState>) -> impl IntoResponse {
    let result = match state.store.list().await {
        Ok(mocks) => (StatusCode::OK, Json(json!(mocks))),
        Err(err) => store_error_response(err),
    };
    result.into_response()
}

async fn create_mock(
    State(state): State<MockServerState>,
    Json(mock): Json<Mock>,
) -> impl IntoResponse {
    if let Err(msg) = mock.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Validation Error", "message": msg })),
        )
            .into_response();
    }

    let result = match state.store.create(mock).await {
        Ok(created) => {
            state.refresh_cache().await;
            (StatusCode::CREATED, Json(json!(created)))
        }
        Err(err) => store_error_response(err),
    };
    result.into_response()
}

async fn get_mock(
    State(state): State<MockServerState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let result = match state.store.get(&id).await {
        Ok(Some(mock)) => (StatusCode::OK, Json(json!(mock))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(
                json!({ "error": "Not Found", "message": format!("Mock with id '{}' not found", id) }),
            ),
        ),
        Err(err) => store_error_response(err),
    };
    result.into_response()
}

async fn update_mock(
    State(state): State<MockServerState>,
    Path(id): Path<String>,
    Json(mock_val): Json<serde_json::Value>,
) -> impl IntoResponse {
    let partial: PartialMock = match serde_json::from_value(mock_val.clone()) {
        Ok(p) => p,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Validation Error", "message": err.to_string() })),
            )
                .into_response();
        }
    };

    if let Some(path) = &partial.path {
        if path.starts_with("/__admin") {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Validation Error", "message": "Field 'path' cannot start with '/__admin'" })),
            )
                .into_response();
        }
    }

    state.sequence_state.reset(&id);

    let result = match state.store.update(&id, partial).await {
        Ok(updated) => {
            state.refresh_cache().await;
            (StatusCode::OK, Json(json!(updated)))
        }
        Err(err) => store_error_response(err),
    };
    result.into_response()
}

async fn delete_mock(
    State(state): State<MockServerState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    state.sequence_state.reset(&id);

    let result = match state.store.delete(&id).await {
        Ok(()) => {
            state.refresh_cache().await;
            (StatusCode::OK, Json(json!({ "success": true })))
        }
        Err(err) => store_error_response(err),
    };
    result.into_response()
}

async fn reset_mock(
    State(state): State<MockServerState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    state.sequence_state.reset(&id);

    if let Ok(Some(mock)) = state.store.get(&id).await {
        if mock.mock_type == MockType::Rest {
            let initial = mock.rest_initial_state.clone().unwrap_or_default();
            let _ = state
                .store
                .update_state(&id, serde_json::to_value(&initial).unwrap())
                .await;
        }
    }

    (StatusCode::OK, Json(json!({ "success": true })))
}

async fn bulk_create_mocks(
    State(state): State<MockServerState>,
    Json(mocks): Json<Vec<Mock>>,
) -> impl IntoResponse {
    let mut created = Vec::new();
    let mut errors = Vec::new();

    for (i, mock) in mocks.into_iter().enumerate() {
        if let Err(msg) = mock.validate() {
            errors.push(json!({ "index": i, "error": msg }));
            continue;
        }
        match state.store.create(mock).await {
            Ok(created_mock) => created.push(created_mock),
            Err(err) => {
                errors.push(json!({ "index": i, "error": err.to_string() }));
            }
        }
    }

    if created.is_empty() && !errors.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "All mocks failed validation", "details": errors })),
        )
            .into_response();
    }

    state.refresh_cache().await;

    let response = if errors.is_empty() {
        json!({ "created": created })
    } else {
        json!({ "created": created, "errors": errors })
    };

    (StatusCode::CREATED, Json(response)).into_response()
}

async fn reset_all_mocks(State(state): State<MockServerState>) -> impl IntoResponse {
    let mocks = match state.store.list().await {
        Ok(m) => m,
        Err(err) => return store_error_response(err).into_response(),
    };

    let mut deleted = 0;
    for mock in &mocks {
        state.sequence_state.reset(&mock.id);
        if let Err(err) = state.store.delete(&mock.id).await {
            tracing::error!(mock_id = %mock.id, error = %err, "Failed to delete mock during reset");
        } else {
            deleted += 1;
        }
    }

    state.refresh_cache().await;

    (
        StatusCode::OK,
        Json(json!({ "success": true, "deleted": deleted })),
    )
        .into_response()
}

async fn livefeed_sse(
    State(state): State<MockServerState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (history, rx) = state.live_feed.subscribe();

    let history_events = history.into_iter().map(|entry| {
        let json_str = serde_json::to_string(&entry).unwrap_or_default();
        Ok(Event::default().data(json_str))
    });
    let history_stream = stream::iter(history_events);

    let live_stream = BroadcastStream::new(rx).filter_map(|msg| async move {
        match msg {
            Ok(entry) => {
                let json_str = serde_json::to_string(&entry).unwrap_or_default();
                Some(Ok(Event::default().data(json_str)))
            }
            Err(_) => None,
        }
    });

    let combined = history_stream.chain(live_stream);

    Sse::new(combined).keep_alive(KeepAlive::default())
}
