use crate::admin::create_admin_router;
use crate::live_feed::LiveFeed;
use crate::matching::find_best_match;
use crate::model::{default_iso_now, FeedEntry, MockRequest, MockType};
use crate::responder::{
    handle_conditional_mock, handle_rest_mock, handle_script_mock, handle_sequence_mock,
    handle_static_mock, SequenceState,
};
use crate::script::ScriptEngine;
use crate::store::MockStore;
use crate::ui;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

#[derive(Clone)]
pub struct MockServerState {
    pub store: Arc<dyn MockStore>,
    pub mock_cache: Arc<tokio::sync::RwLock<Vec<crate::model::Mock>>>,
    pub sequence_state: SequenceState,
    pub script_engine: Option<ScriptEngine>,
    pub live_feed: LiveFeed,
}

impl MockServerState {
    pub async fn refresh_cache(&self) {
        if let Ok(mocks) = self.store.list().await {
            let mut cache = self.mock_cache.write().await;
            *cache = mocks;
        }
    }
}

pub fn create_app_router(state: MockServerState) -> Router {
    Router::new()
        .merge(create_admin_router())
        .route("/", get(ui::redirect_to_ui))
        .route("/ui", get(ui::serve_index))
        .route("/ui/", get(ui::serve_index))
        .route("/ui/index.html", get(ui::serve_index))
        .route("/ui/app.js", get(ui::serve_app_js))
        .route("/ui/styles.css", get(ui::serve_styles_css))
        .route("/favicon.ico", get(ui::serve_favicon_ico))
        .route("/favicon.svg", get(ui::serve_favicon_svg))
        .fallback(handle_mock_request)
        .with_state(state)
}

pub async fn handle_mock_request(State(state): State<MockServerState>, req: Request) -> Response {
    let start = Instant::now();
    let (parts, body) = req.into_parts();

    let method = parts.method.to_string();
    let path = parts.uri.path().to_string();

    let mut query = HashMap::new();
    if let Some(query_str) = parts.uri.query() {
        for (k, v) in
            serde_urlencoded::from_str::<Vec<(String, String)>>(query_str).unwrap_or_default()
        {
            query.insert(k, v);
        }
    }

    let mut headers = HashMap::new();
    for (k, v) in parts.headers.iter() {
        if let Ok(v_str) = v.to_str() {
            headers.insert(k.as_str().to_string(), v_str.to_string());
        }
    }

    let body_bytes = match axum::body::to_bytes(body, 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Bad Request", "message": "Failed to read request body" })),
            )
                .into_response()
        }
    };
    let body_str = String::from_utf8_lossy(&body_bytes).to_string();
    let json_body: Option<serde_json::Value> = serde_json::from_str(&body_str).ok();

    let mut mock_req = MockRequest {
        method: method.clone(),
        path: path.clone(),
        path_params: HashMap::new(),
        headers,
        query,
        body: body_str,
        json_body,
    };

    let mocks = state.mock_cache.read().await.clone();
    let match_res = find_best_match(&mocks, &mock_req);

    if let Some(mock) = match_res.matched_mock {
        mock_req.path_params = match_res.path_params;
        let mock_id = mock.id.clone();
        let mock_name = mock.name.clone();

        let response = match mock.mock_type {
            MockType::Static => handle_static_mock(&mock),
            MockType::Conditional => handle_conditional_mock(&mock, &mock_req),
            MockType::Sequence => handle_sequence_mock(&mock, &state.sequence_state),
            MockType::Script => handle_script_mock(&mock, &mock_req, &state.script_engine).await,
            MockType::Rest => {
                // REST mocks need fresh state from store, not from cache
                let fresh_mock = state
                    .store
                    .get(&mock.id)
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or(mock);
                handle_rest_mock(&fresh_mock, &mock_req, &state.store).await
            }
        };

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        let status = response.status().as_u16();

        state.live_feed.record(FeedEntry {
            id: Uuid::new_v4().to_string(),
            timestamp: default_iso_now(),
            method,
            path,
            matched_mock_id: Some(mock_id),
            matched_mock_name: Some(mock_name),
            status,
            duration_ms,
        });

        response
    } else {
        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

        state.live_feed.record(FeedEntry {
            id: Uuid::new_v4().to_string(),
            timestamp: default_iso_now(),
            method: method.clone(),
            path: path.clone(),
            matched_mock_id: None,
            matched_mock_name: None,
            status: 404,
            duration_ms,
        });

        let payload = json!({
            "error": "Not Found",
            "message": format!("No mock matched for {} {}", method, path)
        });

        (StatusCode::NOT_FOUND, Json(payload)).into_response()
    }
}
