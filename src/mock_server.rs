use crate::admin::create_admin_router;
use crate::live_feed::LiveFeed;
use crate::matching::{MockIndex, find_best_match_indexed};
use crate::model::{FeedEntry, Mock, MockRequest, MockType, default_iso_now};
#[cfg(feature = "script")]
use crate::responder::handle_script_mock;
use crate::responder::{
    SequenceState, handle_conditional_mock, handle_rest_mock, handle_sequence_mock,
    handle_static_mock,
};
#[cfg(feature = "script")]
use crate::script::ScriptEngine;
use crate::store::MockStore;
#[cfg(feature = "ui")]
use crate::ui;
#[cfg(feature = "ui")]
use axum::routing::get;
use axum::{
    Json, Router,
    body::Bytes,
    extract::{Request, State},
    http::{self, StatusCode},
    response::{IntoResponse, Response},
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
    pub mock_index: Arc<tokio::sync::RwLock<MockIndex>>,
    pub sequence_state: SequenceState,
    #[cfg(feature = "script")]
    pub script_engine: Option<ScriptEngine>,
    pub live_feed: LiveFeed,
}

impl MockServerState {
    pub async fn refresh_cache(&self) {
        if let Ok(mocks) = self.store.list().await {
            let mut cache = self.mock_cache.write().await;
            *cache = mocks.clone();
            let mut index = self.mock_index.write().await;
            *index = MockIndex::build(&mocks);
        }
    }
}

pub fn create_app_router(state: MockServerState) -> Router {
    #[cfg(feature = "ui")]
    {
        let mut router = Router::new().merge(create_admin_router());
        router = router
            .route("/", get(ui::redirect_to_ui))
            .route("/ui", get(ui::serve_index))
            .route("/ui/", get(ui::serve_index))
            .route("/ui/index.html", get(ui::serve_index))
            .route("/ui/app.js", get(ui::serve_app_js))
            .route("/ui/styles.css", get(ui::serve_styles_css))
            .route("/favicon.ico", get(ui::serve_favicon_ico))
            .route("/favicon.svg", get(ui::serve_favicon_svg));
        router.fallback(handle_mock_request).with_state(state)
    }
    #[cfg(not(feature = "ui"))]
    {
        Router::new()
            .merge(create_admin_router())
            .fallback(handle_mock_request)
            .with_state(state)
    }
}

/// Parse the incoming HTTP request into a MockRequest.
///
/// Extracts query parameters, headers, and body from the request parts.
fn parse_mock_request(parts: &http::request::Parts, body_bytes: &Bytes) -> MockRequest {
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

    let body_str = String::from_utf8_lossy(body_bytes).to_string();
    let json_body: Option<serde_json::Value> = serde_json::from_str(&body_str).ok();

    MockRequest {
        method,
        path,
        path_params: HashMap::new(),
        headers,
        query,
        body: body_str,
        json_body,
    }
}

/// Dispatch to the appropriate handler based on the mock type.
///
/// Routes Static, Conditional, Sequence, Script, and Rest mocks to their
/// respective handlers.
async fn dispatch_mock_response(
    mock: &Mock,
    mock_req: &MockRequest,
    state: &MockServerState,
) -> Response {
    match mock.mock_type {
        MockType::Static => handle_static_mock(mock),
        MockType::Conditional => handle_conditional_mock(mock, mock_req),
        MockType::Sequence => handle_sequence_mock(mock, &state.sequence_state),
        #[cfg(feature = "script")]
        MockType::Script => handle_script_mock(mock, mock_req, &state.script_engine).await,
        #[cfg(not(feature = "script"))]
        MockType::Script => {
            let err_msg = "Script mock type is not available (script feature disabled)";
            axum::http::Response::builder()
                .status(axum::http::StatusCode::NOT_IMPLEMENTED)
                .header("content-type", "text/plain")
                .body(axum::body::Body::from(err_msg))
                .unwrap()
        }
        MockType::Rest => {
            // REST mocks need fresh state from store, not from cache
            let fresh_mock = state
                .store
                .get(&mock.id)
                .await
                .ok()
                .flatten()
                .unwrap_or(mock.clone());
            handle_rest_mock(&fresh_mock, mock_req, &state.store).await
        }
    }
}

/// Record a feed entry for a completed request.
///
/// Creates a FeedEntry with timing and match information and records it
/// in the live feed.
fn record_feed_entry(
    live_feed: &LiveFeed,
    method: String,
    path: String,
    matched_mock_id: Option<String>,
    matched_mock_name: Option<String>,
    status: u16,
    duration_ms: f64,
) {
    live_feed.record(FeedEntry {
        id: Uuid::new_v4().to_string(),
        timestamp: default_iso_now(),
        method,
        path,
        matched_mock_id,
        matched_mock_name,
        status,
        duration_ms,
    });
}

pub async fn handle_mock_request(State(state): State<MockServerState>, req: Request) -> Response {
    let start = Instant::now();
    let (parts, body) = req.into_parts();

    let method = parts.method.to_string();
    let path = parts.uri.path().to_string();

    let body_bytes = match axum::body::to_bytes(body, 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Bad Request", "message": "Failed to read request body" })),
            )
                .into_response();
        }
    };

    let mut mock_req = parse_mock_request(&parts, &body_bytes);

    let index = state.mock_index.read().await;
    let match_res = find_best_match_indexed(&index, &mock_req);

    if let Some(mock) = match_res.matched_mock {
        mock_req.path_params = match_res.path_params;
        let mock_id = mock.id.clone();
        let mock_name = mock.name.clone();

        let response = dispatch_mock_response(&mock, &mock_req, &state).await;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        let status = response.status().as_u16();

        record_feed_entry(
            &state.live_feed,
            method,
            path,
            Some(mock_id),
            Some(mock_name),
            status,
            duration_ms,
        );

        response
    } else {
        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

        record_feed_entry(
            &state.live_feed,
            method.clone(),
            path.clone(),
            None,
            None,
            404,
            duration_ms,
        );

        let payload = json!({
            "error": "Not Found",
            "message": format!("No mock matched for {} {}", method, path)
        });

        (StatusCode::NOT_FOUND, Json(payload)).into_response()
    }
}
