use crate::model::{ConditionOp, ConditionSource, Mock, MockRequest, MockResponse, SequenceMode};
use crate::script::ScriptEngine;
use crate::store::MockStore;
use axum::{
    body::Body,
    http::{HeaderName, HeaderValue, Response, StatusCode},
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct SequenceState {
    counters: Arc<RwLock<HashMap<String, usize>>>,
}

impl SequenceState {
    pub fn new() -> Self {
        Self {
            counters: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn next_response(
        &self,
        mock_id: &str,
        sequence: &[MockResponse],
        mode: Option<&SequenceMode>,
    ) -> MockResponse {
        if sequence.is_empty() {
            return MockResponse::default();
        }

        let mode = mode.unwrap_or(&SequenceMode::Cycle);
        let mut guard = self.counters.write().unwrap();
        let idx = guard.entry(mock_id.to_string()).or_insert(0);
        let current = *idx;

        let next = match mode {
            SequenceMode::Cycle => (current + 1) % sequence.len(),
            SequenceMode::StopAtEnd => {
                if current + 1 < sequence.len() {
                    current + 1
                } else {
                    current
                }
            }
        };

        *idx = next;
        sequence[current.min(sequence.len() - 1)].clone()
    }

    pub fn reset(&self, mock_id: &str) {
        let mut guard = self.counters.write().unwrap();
        guard.remove(mock_id);
    }
}

pub fn build_mock_response(spec: &MockResponse) -> Response<Body> {
    let status = StatusCode::from_u16(spec.status).unwrap_or(StatusCode::OK);
    let mut builder = Response::builder().status(status);

    let mut has_content_type = false;
    if let Some(headers) = &spec.headers {
        for (k, v) in headers {
            if k.eq_ignore_ascii_case("content-type") {
                has_content_type = true;
            }
            if let (Ok(name), Ok(val)) = (HeaderName::try_from(k), HeaderValue::try_from(v)) {
                builder = builder.header(name, val);
            }
        }
    }

    if !has_content_type {
        if let Some(ct) = &spec.content_type {
            if let (Ok(name), Ok(val)) = (
                HeaderName::try_from("content-type"),
                HeaderValue::try_from(ct),
            ) {
                builder = builder.header(name, val);
            }
        } else {
            builder = builder.header("content-type", "text/plain");
        }
    }

    let body_str = spec.body.clone().unwrap_or_default();
    builder
        .body(Body::from(body_str))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

pub fn handle_static_mock(mock: &Mock) -> Response<Body> {
    let spec = mock.response.as_ref().cloned().unwrap_or_default();
    build_mock_response(&spec)
}

pub fn handle_conditional_mock(mock: &Mock, req: &MockRequest) -> Response<Body> {
    let conditions = mock.conditions.as_deref().unwrap_or(&[]);

    for condition in conditions {
        let actual_value = match condition.source {
            ConditionSource::Header => req.headers.iter().find_map(|(k, v)| {
                if k.eq_ignore_ascii_case(&condition.key) {
                    Some(v.clone())
                } else {
                    None
                }
            }),
            ConditionSource::Query => req.query.get(&condition.key).cloned(),
            ConditionSource::Body => {
                if let Some(serde_json::Value::Object(map)) = &req.json_body {
                    map.get(&condition.key).and_then(|v| {
                        if v.is_null() {
                            None
                        } else if let Some(s) = v.as_str() {
                            Some(s.to_string())
                        } else {
                            Some(v.to_string())
                        }
                    })
                } else {
                    None
                }
            }
        };

        let matches = match condition.op {
            ConditionOp::Exists => actual_value.is_some(),
            ConditionOp::NotExists => actual_value.is_none(),
            ConditionOp::Equals => {
                let expected = condition.value.as_deref().unwrap_or("");
                actual_value.as_deref() == Some(expected)
            }
            ConditionOp::NotEquals => {
                let expected = condition.value.as_deref().unwrap_or("");
                actual_value.as_deref() != Some(expected)
            }
        };

        if matches {
            return build_mock_response(&condition.response);
        }
    }

    // No condition matched
    if let Some(else_resp) = &mock.else_response {
        build_mock_response(else_resp)
    } else {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("content-type", "text/plain")
            .body(Body::from(
                "No condition matched and no else response configured",
            ))
            .unwrap()
    }
}

pub fn handle_sequence_mock(mock: &Mock, sequence_state: &SequenceState) -> Response<Body> {
    let sequence = mock.sequence.as_deref().unwrap_or(&[]);
    let resp = sequence_state.next_response(&mock.id, sequence, mock.sequence_mode.as_ref());
    build_mock_response(&resp)
}

pub async fn handle_script_mock(
    mock: &Mock,
    req: &MockRequest,
    script_engine: &Option<ScriptEngine>,
) -> Response<Body> {
    if let Some(engine) = script_engine {
        let script = mock.script.as_deref().unwrap_or("");
        match engine.execute(script, mock.script_timeout_ms, req).await {
            Ok(resp) => build_mock_response(&resp),
            Err(err) => {
                let err_msg = if err.starts_with("Script error: ") {
                    err
                } else {
                    format!("Script error: {}", err)
                };
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header("content-type", "text/plain")
                    .body(Body::from(err_msg))
                    .unwrap()
            }
        }
    } else {
        Response::builder()
            .status(StatusCode::NOT_IMPLEMENTED)
            .header("content-type", "text/plain")
            .body(Body::from("Script engine not configured"))
            .unwrap()
    }
}

pub async fn handle_rest_mock(
    mock: &Mock,
    req: &MockRequest,
    store: &Arc<dyn MockStore>,
) -> Response<Body> {
    let id_field = mock.rest_id_field.as_deref().unwrap_or("id");
    let mut items = mock.state.clone().unwrap_or_default();

    if let Some(target_id) = req
        .path_params
        .get("id")
        .or_else(|| req.path_params.get(id_field))
    {
        // Item operation
        match req.method.to_uppercase().as_str() {
            "GET" => {
                let found = items
                    .iter()
                    .find(|item| item_matches_id(item, id_field, target_id));
                match found {
                    Some(item) => json_response(StatusCode::OK, item),
                    None => {
                        not_found_json(&format!("Item with {} '{}' not found", id_field, target_id))
                    }
                }
            }
            "PUT" => {
                let body_json: serde_json::Value =
                    match serde_json::from_str::<serde_json::Value>(&req.body) {
                        Ok(val) if val.is_object() => val,
                        _ => return bad_request_json("Request body must be a JSON object"),
                    };

                let idx = items
                    .iter()
                    .position(|item| item_matches_id(item, id_field, target_id));
                match idx {
                    Some(i) => {
                        let mut new_obj = body_json;
                        if let serde_json::Value::Object(ref mut map) = new_obj {
                            map.insert(
                                id_field.to_string(),
                                serde_json::Value::String(target_id.to_string()),
                            );
                        }
                        items[i] = new_obj.clone();
                        let _ = store
                            .update_state(&mock.id, serde_json::to_value(&items).unwrap())
                            .await;
                        json_response(StatusCode::OK, &new_obj)
                    }
                    None => {
                        not_found_json(&format!("Item with {} '{}' not found", id_field, target_id))
                    }
                }
            }
            "PATCH" => {
                let body_json: serde_json::Value =
                    match serde_json::from_str::<serde_json::Value>(&req.body) {
                        Ok(val) if val.is_object() => val,
                        _ => return bad_request_json("Request body must be a JSON object"),
                    };

                let idx = items
                    .iter()
                    .position(|item| item_matches_id(item, id_field, target_id));
                match idx {
                    Some(i) => {
                        let mut existing = items[i].clone();
                        if let (
                            serde_json::Value::Object(ref mut ex_map),
                            serde_json::Value::Object(patch_map),
                        ) = (&mut existing, body_json)
                        {
                            for (k, v) in patch_map {
                                ex_map.insert(k, v);
                            }
                            ex_map.insert(
                                id_field.to_string(),
                                serde_json::Value::String(target_id.to_string()),
                            );
                        }
                        items[i] = existing.clone();
                        let _ = store
                            .update_state(&mock.id, serde_json::to_value(&items).unwrap())
                            .await;
                        json_response(StatusCode::OK, &existing)
                    }
                    None => {
                        not_found_json(&format!("Item with {} '{}' not found", id_field, target_id))
                    }
                }
            }
            "DELETE" => {
                let idx = items
                    .iter()
                    .position(|item| item_matches_id(item, id_field, target_id));
                if let Some(i) = idx {
                    items.remove(i);
                    let _ = store
                        .update_state(&mock.id, serde_json::to_value(&items).unwrap())
                        .await;
                }
                Response::builder()
                    .status(StatusCode::NO_CONTENT)
                    .body(Body::empty())
                    .unwrap()
            }
            _ => method_not_allowed_json(),
        }
    } else {
        // Collection operation
        match req.method.to_uppercase().as_str() {
            "GET" => json_response(StatusCode::OK, &items),
            "POST" => {
                let mut body_json: serde_json::Value =
                    match serde_json::from_str::<serde_json::Value>(&req.body) {
                        Ok(val) if val.is_object() => val,
                        _ => return bad_request_json("Request body must be a JSON object"),
                    };

                if let serde_json::Value::Object(ref mut map) = body_json {
                    let has_id = map
                        .get(id_field)
                        .map(|v| !v.is_null() && v.as_str() != Some(""))
                        .unwrap_or(false);
                    if !has_id {
                        map.insert(
                            id_field.to_string(),
                            serde_json::Value::String(Uuid::new_v4().to_string()),
                        );
                    }
                }

                items.push(body_json.clone());
                let _ = store
                    .update_state(&mock.id, serde_json::to_value(&items).unwrap())
                    .await;
                json_response(StatusCode::CREATED, &body_json)
            }
            "PUT" => {
                let body_json: serde_json::Value =
                    match serde_json::from_str::<serde_json::Value>(&req.body) {
                        Ok(val) if val.is_array() => val,
                        _ => return bad_request_json("Request body must be a JSON array"),
                    };

                items = serde_json::from_value(body_json).unwrap_or_default();
                let _ = store
                    .update_state(&mock.id, serde_json::to_value(&items).unwrap())
                    .await;
                json_response(StatusCode::OK, &items)
            }
            "DELETE" => {
                items.clear();
                let _ = store
                    .update_state(&mock.id, serde_json::to_value(&items).unwrap())
                    .await;
                Response::builder()
                    .status(StatusCode::NO_CONTENT)
                    .body(Body::empty())
                    .unwrap()
            }
            _ => method_not_allowed_json(),
        }
    }
}

fn item_matches_id(item: &serde_json::Value, id_field: &str, target_id: &str) -> bool {
    if let serde_json::Value::Object(map) = item {
        if let Some(val) = map.get(id_field) {
            if let Some(s) = val.as_str() {
                return s == target_id;
            } else if val.is_number() {
                return val == target_id;
            }
        }
    }
    false
}

fn json_response<T: serde::Serialize>(status: StatusCode, data: &T) -> Response<Body> {
    let body_str = serde_json::to_string(data).unwrap_or_default();
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(body_str))
        .unwrap()
}

fn not_found_json(msg: &str) -> Response<Body> {
    let payload = json!({
        "error": "Not Found",
        "message": msg
    });
    json_response(StatusCode::NOT_FOUND, &payload)
}

fn bad_request_json(msg: &str) -> Response<Body> {
    let payload = json!({
        "error": "Bad Request",
        "message": msg
    });
    json_response(StatusCode::BAD_REQUEST, &payload)
}

fn method_not_allowed_json() -> Response<Body> {
    let payload = json!({
        "error": "Method Not Allowed"
    });
    json_response(StatusCode::METHOD_NOT_ALLOWED, &payload)
}
