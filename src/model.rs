use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MockType {
    Static,
    Conditional,
    Sequence,
    Script,
    Rest,
}

impl std::fmt::Display for MockType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MockType::Static => write!(f, "static"),
            MockType::Conditional => write!(f, "conditional"),
            MockType::Sequence => write!(f, "sequence"),
            MockType::Script => write!(f, "script"),
            MockType::Rest => write!(f, "rest"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub enum SequenceMode {
    #[default]
    Cycle,
    StopAtEnd,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MockResponse {
    #[serde(default = "default_status")]
    pub status: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(
        default,
        rename = "contentType",
        skip_serializing_if = "Option::is_none"
    )]
    pub content_type: Option<String>,
}

impl Default for MockResponse {
    fn default() -> Self {
        Self {
            status: 200,
            headers: None,
            body: None,
            content_type: None,
        }
    }
}

fn default_status() -> u16 {
    200
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConditionSource {
    Header,
    Query,
    Body,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ConditionOp {
    Exists,
    NotExists,
    Equals,
    NotEquals,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Condition {
    pub source: ConditionSource,
    pub key: String,
    pub op: ConditionOp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    pub response: MockResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Mock {
    #[serde(default = "default_uuid")]
    pub id: String,
    pub name: String,
    pub method: String,
    pub path: String,
    #[serde(rename = "type")]
    pub mock_type: MockType,
    #[serde(default)]
    pub priority: i32,

    // Static Mock fields
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<MockResponse>,

    // Conditional Mock fields
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Vec<Condition>>,
    #[serde(
        default,
        rename = "elseResponse",
        skip_serializing_if = "Option::is_none"
    )]
    pub else_response: Option<MockResponse>,

    // Sequence Mock fields
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<Vec<MockResponse>>,
    #[serde(
        default,
        rename = "sequenceMode",
        skip_serializing_if = "Option::is_none"
    )]
    pub sequence_mode: Option<SequenceMode>,

    // Script Mock fields
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    #[serde(
        default,
        rename = "scriptTimeoutMs",
        skip_serializing_if = "Option::is_none"
    )]
    pub script_timeout_ms: Option<u64>,

    // REST Mock fields
    #[serde(
        default,
        rename = "restIdField",
        skip_serializing_if = "Option::is_none"
    )]
    pub rest_id_field: Option<String>,
    #[serde(
        default,
        rename = "restInitialState",
        skip_serializing_if = "Option::is_none"
    )]
    pub rest_initial_state: Option<Vec<serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<Vec<serde_json::Value>>,

    #[serde(default = "default_iso_now", rename = "createdAt")]
    pub created_at: String,
    #[serde(default = "default_iso_now", rename = "updatedAt")]
    pub updated_at: String,
}

fn default_uuid() -> String {
    Uuid::new_v4().to_string()
}

pub fn default_iso_now() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// Builder pattern for fluent construction of `Mock`.
///
/// Allows building mocks in a cleaner way, especially in tests,
/// automatically filling missing fields with default values.
///
/// # Example
///
/// ```no_run
/// use done_mock::model::{Mock, MockType, MockResponse};
///
/// let mock = Mock::builder()
///     .name("Test")
///     .method("GET")
///     .path("/test")
///     .mock_type(MockType::Static)
///     .response(MockResponse {
///         status: 200,
///         body: Some("ok".to_string()),
///         ..Default::default()
///     })
///     .build();
/// ```
pub struct MockBuilder {
    id: Option<String>,
    name: Option<String>,
    method: Option<String>,
    path: Option<String>,
    mock_type: Option<MockType>,
    priority: Option<i32>,
    response: Option<MockResponse>,
    conditions: Option<Vec<Condition>>,
    else_response: Option<MockResponse>,
    sequence: Option<Vec<MockResponse>>,
    sequence_mode: Option<SequenceMode>,
    script: Option<String>,
    script_timeout_ms: Option<u64>,
    rest_id_field: Option<String>,
    rest_initial_state: Option<Vec<serde_json::Value>>,
    state: Option<Vec<serde_json::Value>>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

impl MockBuilder {
    /// Creates an empty builder.
    pub fn new() -> Self {
        Self {
            id: None,
            name: None,
            method: None,
            path: None,
            mock_type: None,
            priority: None,
            response: None,
            conditions: None,
            else_response: None,
            sequence: None,
            sequence_mode: None,
            script: None,
            script_timeout_ms: None,
            rest_id_field: None,
            rest_initial_state: None,
            state: None,
            created_at: None,
            updated_at: None,
        }
    }

    /// Sets the mock's ID. If not provided, a UUID v4 is generated automatically.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the mock's name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the mock's HTTP method.
    pub fn method(mut self, method: impl Into<String>) -> Self {
        self.method = Some(method.into());
        self
    }

    /// Sets the mock's path.
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Sets the mock's type (Static, Conditional, Sequence, Script, Rest).
    pub fn mock_type(mut self, mock_type: MockType) -> Self {
        self.mock_type = Some(mock_type);
        self
    }

    /// Sets the mock's priority.
    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Sets the mock's static response.
    pub fn response(mut self, response: MockResponse) -> Self {
        self.response = Some(response);
        self
    }

    /// Sets the mock's conditions.
    pub fn conditions(mut self, conditions: Vec<Condition>) -> Self {
        self.conditions = Some(conditions);
        self
    }

    /// Sets the conditional mock's default (else) response.
    pub fn else_response(mut self, else_response: MockResponse) -> Self {
        self.else_response = Some(else_response);
        self
    }

    /// Sets the mock's sequence of responses.
    pub fn sequence(mut self, sequence: Vec<MockResponse>) -> Self {
        self.sequence = Some(sequence);
        self
    }

    /// Sets the sequence mode (Cycle, StopAtEnd).
    pub fn sequence_mode(mut self, sequence_mode: SequenceMode) -> Self {
        self.sequence_mode = Some(sequence_mode);
        self
    }

    /// Sets the mock's JavaScript script.
    pub fn script(mut self, script: impl Into<String>) -> Self {
        self.script = Some(script.into());
        self
    }

    /// Sets the script timeout in milliseconds.
    pub fn script_timeout_ms(mut self, script_timeout_ms: u64) -> Self {
        self.script_timeout_ms = Some(script_timeout_ms);
        self
    }

    /// Sets the ID field for REST mocks.
    pub fn rest_id_field(mut self, rest_id_field: impl Into<String>) -> Self {
        self.rest_id_field = Some(rest_id_field.into());
        self
    }

    /// Sets the initial state for REST mocks.
    pub fn rest_initial_state(mut self, rest_initial_state: Vec<serde_json::Value>) -> Self {
        self.rest_initial_state = Some(rest_initial_state);
        self
    }

    /// Sets the current state for REST mocks.
    pub fn state(mut self, state: Vec<serde_json::Value>) -> Self {
        self.state = Some(state);
        self
    }

    /// Sets the creation timestamp. If not provided, uses the current time.
    pub fn created_at(mut self, created_at: impl Into<String>) -> Self {
        self.created_at = Some(created_at.into());
        self
    }

    /// Sets the update timestamp. If not provided, uses the current time.
    pub fn updated_at(mut self, updated_at: impl Into<String>) -> Self {
        self.updated_at = Some(updated_at.into());
        self
    }

    /// Builds the `Mock` from the configured values.
    ///
    /// Required fields (name, method, path, mock_type) should be set
    /// before calling `build()`. Missing fields receive default values:
    /// - `id`: Auto-generated UUID v4
    /// - `priority`: 0
    /// - `created_at`/`updated_at`: Current timestamp
    /// - Optional fields: `None`
    pub fn build(self) -> Mock {
        let now = default_iso_now();
        Mock {
            id: self.id.unwrap_or_else(default_uuid),
            name: self.name.unwrap_or_default(),
            method: self.method.unwrap_or_default(),
            path: self.path.unwrap_or_default(),
            mock_type: self.mock_type.unwrap_or(MockType::Static),
            priority: self.priority.unwrap_or(0),
            response: self.response,
            conditions: self.conditions,
            else_response: self.else_response,
            sequence: self.sequence,
            sequence_mode: self.sequence_mode,
            script: self.script,
            script_timeout_ms: self.script_timeout_ms,
            rest_id_field: self.rest_id_field,
            rest_initial_state: self.rest_initial_state,
            state: self.state,
            created_at: self.created_at.unwrap_or_else(|| now.clone()),
            updated_at: self.updated_at.unwrap_or(now),
        }
    }
}

impl Default for MockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Mock {
    /// Returns a `MockBuilder` for fluent construction.
    pub fn builder() -> MockBuilder {
        MockBuilder::new()
    }

    /// Applies the fields present in `PartialMock` to this `Mock`,
    /// updating the `updated_at` timestamp.
    pub fn apply_partial(&mut self, partial: PartialMock) {
        if let Some(name) = partial.name {
            self.name = name;
        }
        if let Some(method) = partial.method {
            self.method = method;
        }
        if let Some(path) = partial.path {
            self.path = path;
        }
        if let Some(mock_type) = partial.mock_type {
            self.mock_type = mock_type;
        }
        if let Some(priority) = partial.priority {
            self.priority = priority;
        }
        if partial.response.is_some() {
            self.response = partial.response;
        }
        if partial.conditions.is_some() {
            self.conditions = partial.conditions;
        }
        if partial.else_response.is_some() {
            self.else_response = partial.else_response;
        }
        if partial.sequence.is_some() {
            self.sequence = partial.sequence;
        }
        if partial.sequence_mode.is_some() {
            self.sequence_mode = partial.sequence_mode;
        }
        if partial.script.is_some() {
            self.script = partial.script;
        }
        if partial.script_timeout_ms.is_some() {
            self.script_timeout_ms = partial.script_timeout_ms;
        }
        if partial.rest_id_field.is_some() {
            self.rest_id_field = partial.rest_id_field;
        }
        if partial.rest_initial_state.is_some() {
            self.rest_initial_state = partial.rest_initial_state;
        }
        if partial.state.is_some() {
            self.state = partial.state;
        }

        self.updated_at = default_iso_now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PartialMock {
    pub name: Option<String>,
    pub method: Option<String>,
    pub path: Option<String>,
    #[serde(rename = "type")]
    pub mock_type: Option<MockType>,
    pub priority: Option<i32>,
    pub response: Option<MockResponse>,
    pub conditions: Option<Vec<Condition>>,
    #[serde(rename = "elseResponse")]
    pub else_response: Option<MockResponse>,
    pub sequence: Option<Vec<MockResponse>>,
    #[serde(rename = "sequenceMode")]
    pub sequence_mode: Option<SequenceMode>,
    pub script: Option<String>,
    #[serde(rename = "scriptTimeoutMs")]
    pub script_timeout_ms: Option<u64>,
    #[serde(rename = "restIdField")]
    pub rest_id_field: Option<String>,
    #[serde(rename = "restInitialState")]
    pub rest_initial_state: Option<Vec<serde_json::Value>>,
    pub state: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedEntry {
    pub id: String,
    pub timestamp: String,
    pub method: String,
    pub path: String,
    #[serde(rename = "matchedMockId")]
    pub matched_mock_id: Option<String>,
    #[serde(rename = "matchedMockName")]
    pub matched_mock_name: Option<String>,
    pub status: u16,
    #[serde(rename = "durationMs")]
    pub duration_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockRequest {
    pub method: String,
    pub path: String,
    pub path_params: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub query: HashMap<String, String>,
    pub body: String,
    pub json_body: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- MockType Display tests ---

    #[test]
    fn test_mock_type_display_static() {
        assert_eq!(format!("{}", MockType::Static), "static");
    }

    #[test]
    fn test_mock_type_display_conditional() {
        assert_eq!(format!("{}", MockType::Conditional), "conditional");
    }

    #[test]
    fn test_mock_type_display_sequence() {
        assert_eq!(format!("{}", MockType::Sequence), "sequence");
    }

    #[test]
    fn test_mock_type_display_script() {
        assert_eq!(format!("{}", MockType::Script), "script");
    }

    #[test]
    fn test_mock_type_display_rest() {
        assert_eq!(format!("{}", MockType::Rest), "rest");
    }

    // --- MockResponse tests ---

    #[test]
    fn test_mock_response_default() {
        let resp = MockResponse::default();
        assert_eq!(resp.status, 200);
        assert!(resp.headers.is_none());
        assert!(resp.body.is_none());
        assert!(resp.content_type.is_none());
    }

    #[test]
    fn test_mock_response_serialization() {
        let resp = MockResponse {
            status: 201,
            headers: None,
            body: Some("created".to_string()),
            content_type: Some("text/plain".to_string()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"status\":201"));
        assert!(json.contains("\"body\":\"created\""));
        assert!(json.contains("\"contentType\":\"text/plain\""));
    }

    #[test]
    fn test_mock_response_deserialization_defaults() {
        let json = r#"{}"#;
        let resp: MockResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.status, 200);
        assert!(resp.body.is_none());
    }

    #[test]
    fn test_mock_response_round_trip() {
        let mut headers = HashMap::new();
        headers.insert("X-Custom".to_string(), "value".to_string());
        let resp = MockResponse {
            status: 201,
            headers: Some(headers),
            body: Some("data".to_string()),
            content_type: Some("application/json".to_string()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: MockResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.status, 201);
        assert_eq!(deserialized.body, Some("data".to_string()));
        assert_eq!(
            deserialized.content_type,
            Some("application/json".to_string())
        );
    }

    // --- SequenceMode tests ---

    #[test]
    fn test_sequence_mode_default() {
        let mode: SequenceMode = Default::default();
        assert_eq!(mode, SequenceMode::Cycle);
    }

    #[test]
    fn test_sequence_mode_serialization() {
        let json = serde_json::to_string(&SequenceMode::Cycle).unwrap();
        assert_eq!(json, "\"cycle\"");

        let json = serde_json::to_string(&SequenceMode::StopAtEnd).unwrap();
        assert_eq!(json, "\"stopAtEnd\"");
    }

    #[test]
    fn test_sequence_mode_deserialization() {
        let mode: SequenceMode = serde_json::from_str("\"cycle\"").unwrap();
        assert_eq!(mode, SequenceMode::Cycle);

        let mode: SequenceMode = serde_json::from_str("\"stopAtEnd\"").unwrap();
        assert_eq!(mode, SequenceMode::StopAtEnd);
    }

    // --- Condition tests ---

    #[test]
    fn test_condition_serialization() {
        let cond = Condition {
            source: ConditionSource::Header,
            key: "X-Test".to_string(),
            op: ConditionOp::Equals,
            value: Some("val".to_string()),
            response: MockResponse::default(),
        };
        let json = serde_json::to_string(&cond).unwrap();
        assert!(json.contains("\"source\":\"header\""));
        assert!(json.contains("\"key\":\"X-Test\""));
        assert!(json.contains("\"op\":\"equals\""));
    }

    // --- Mock serialization tests ---

    // --- MockBuilder tests ---

    #[test]
    fn test_builder_static_mock() {
        let mock = Mock::builder()
            .id("1")
            .name("Test")
            .method("GET")
            .path("/test")
            .mock_type(MockType::Static)
            .priority(5)
            .response(MockResponse {
                status: 200,
                headers: None,
                body: Some("ok".to_string()),
                content_type: None,
            })
            .created_at("2026-01-01T00:00:00.000Z")
            .updated_at("2026-01-01T00:00:00.000Z")
            .build();

        assert_eq!(mock.id, "1");
        assert_eq!(mock.name, "Test");
        assert_eq!(mock.method, "GET");
        assert_eq!(mock.path, "/test");
        assert_eq!(mock.mock_type, MockType::Static);
        assert_eq!(mock.priority, 5);
        assert!(mock.response.is_some());
        assert!(mock.conditions.is_none());
        assert!(mock.sequence.is_none());
    }

    #[test]
    fn test_builder_generates_defaults() {
        let mock = Mock::builder()
            .name("Minimal")
            .method("POST")
            .path("/api")
            .build();

        assert!(!mock.id.is_empty());
        assert_eq!(mock.name, "Minimal");
        assert_eq!(mock.method, "POST");
        assert_eq!(mock.path, "/api");
        assert_eq!(mock.mock_type, MockType::Static);
        assert_eq!(mock.priority, 0);
        assert!(mock.created_at.len() > 0);
        assert!(mock.updated_at.len() > 0);
    }

    #[test]
    fn test_builder_conditional_mock() {
        let mock = Mock::builder()
            .name("Conditional")
            .method("GET")
            .path("/test")
            .mock_type(MockType::Conditional)
            .conditions(vec![Condition {
                source: ConditionSource::Header,
                key: "X-V".to_string(),
                op: ConditionOp::Equals,
                value: Some("1".to_string()),
                response: MockResponse::default(),
            }])
            .else_response(MockResponse {
                status: 404,
                headers: None,
                body: None,
                content_type: None,
            })
            .build();

        assert_eq!(mock.mock_type, MockType::Conditional);
        assert!(mock.conditions.is_some());
        assert!(mock.else_response.is_some());
    }

    #[test]
    fn test_builder_sequence_mock() {
        let mock = Mock::builder()
            .name("Sequence")
            .method("GET")
            .path("/test")
            .mock_type(MockType::Sequence)
            .sequence(vec![
                MockResponse {
                    status: 200,
                    headers: None,
                    body: Some("first".to_string()),
                    content_type: None,
                },
                MockResponse {
                    status: 201,
                    headers: None,
                    body: Some("second".to_string()),
                    content_type: None,
                },
            ])
            .sequence_mode(SequenceMode::StopAtEnd)
            .build();

        assert_eq!(mock.mock_type, MockType::Sequence);
        assert!(mock.sequence.is_some());
        assert_eq!(mock.sequence_mode, Some(SequenceMode::StopAtEnd));
    }

    #[test]
    fn test_builder_script_mock() {
        let mock = Mock::builder()
            .name("Script")
            .method("GET")
            .path("/test")
            .mock_type(MockType::Script)
            .script("return { status: 200 }")
            .script_timeout_ms(5000)
            .build();

        assert_eq!(mock.mock_type, MockType::Script);
        assert_eq!(mock.script, Some("return { status: 200 }".to_string()));
        assert_eq!(mock.script_timeout_ms, Some(5000));
    }

    #[test]
    fn test_builder_rest_mock() {
        let mock = Mock::builder()
            .name("REST")
            .method("GET")
            .path("/items")
            .mock_type(MockType::Rest)
            .rest_id_field("itemId")
            .rest_initial_state(vec![serde_json::json!({"id": "1", "name": "Item"})])
            .build();

        assert_eq!(mock.mock_type, MockType::Rest);
        assert_eq!(mock.rest_id_field, Some("itemId".to_string()));
        assert!(mock.rest_initial_state.is_some());
    }

    #[test]
    fn test_builder_new() {
        let builder = MockBuilder::new();
        let mock = builder
            .name("From new()")
            .method("GET")
            .path("/test")
            .build();
        assert_eq!(mock.name, "From new()");
    }

    // --- Mock serialization tests ---

    #[test]
    fn test_mock_serialization_static() {
        let mock = Mock::builder()
            .id("1")
            .name("Test")
            .method("GET")
            .path("/test")
            .mock_type(MockType::Static)
            .priority(5)
            .response(MockResponse {
                status: 200,
                headers: None,
                body: Some("ok".to_string()),
                content_type: None,
            })
            .created_at("2026-01-01T00:00:00.000Z")
            .updated_at("2026-01-01T00:00:00.000Z")
            .build();
        let json = serde_json::to_string(&mock).unwrap();
        assert!(json.contains("\"type\":\"static\""));
        assert!(json.contains("\"name\":\"Test\""));
        assert!(json.contains("\"method\":\"GET\""));
        assert!(json.contains("\"path\":\"/test\""));
        assert!(json.contains("\"priority\":5"));
    }

    #[test]
    fn test_mock_serialization_conditional() {
        let mock = Mock::builder()
            .id("1")
            .name("Conditional")
            .method("GET")
            .path("/test")
            .mock_type(MockType::Conditional)
            .conditions(vec![Condition {
                source: ConditionSource::Header,
                key: "X-V".to_string(),
                op: ConditionOp::Equals,
                value: Some("1".to_string()),
                response: MockResponse::default(),
            }])
            .else_response(MockResponse {
                status: 404,
                headers: None,
                body: None,
                content_type: None,
            })
            .created_at("2026-01-01T00:00:00.000Z")
            .updated_at("2026-01-01T00:00:00.000Z")
            .build();
        let json = serde_json::to_string(&mock).unwrap();
        assert!(json.contains("\"type\":\"conditional\""));
        assert!(json.contains("\"elseResponse\""));
    }

    #[test]
    fn test_mock_serialization_sequence() {
        let mock = Mock::builder()
            .id("1")
            .name("Sequence")
            .method("GET")
            .path("/test")
            .mock_type(MockType::Sequence)
            .sequence(vec![
                MockResponse {
                    status: 200,
                    headers: None,
                    body: Some("first".to_string()),
                    content_type: None,
                },
                MockResponse {
                    status: 201,
                    headers: None,
                    body: Some("second".to_string()),
                    content_type: None,
                },
            ])
            .sequence_mode(SequenceMode::StopAtEnd)
            .created_at("2026-01-01T00:00:00.000Z")
            .updated_at("2026-01-01T00:00:00.000Z")
            .build();
        let json = serde_json::to_string(&mock).unwrap();
        assert!(json.contains("\"type\":\"sequence\""));
        assert!(json.contains("\"sequenceMode\":\"stopAtEnd\""));
    }

    #[test]
    fn test_mock_serialization_script() {
        let mock = Mock::builder()
            .id("1")
            .name("Script")
            .method("GET")
            .path("/test")
            .mock_type(MockType::Script)
            .script("return { status: 200 }")
            .script_timeout_ms(5000)
            .created_at("2026-01-01T00:00:00.000Z")
            .updated_at("2026-01-01T00:00:00.000Z")
            .build();
        let json = serde_json::to_string(&mock).unwrap();
        assert!(json.contains("\"type\":\"script\""));
        assert!(json.contains("\"scriptTimeoutMs\":5000"));
    }

    #[test]
    fn test_mock_serialization_rest() {
        let mock = Mock::builder()
            .id("1")
            .name("REST")
            .method("GET")
            .path("/items")
            .mock_type(MockType::Rest)
            .rest_id_field("itemId")
            .rest_initial_state(vec![serde_json::json!({"id": "1", "name": "Item"})])
            .created_at("2026-01-01T00:00:00.000Z")
            .updated_at("2026-01-01T00:00:00.000Z")
            .build();
        let json = serde_json::to_string(&mock).unwrap();
        assert!(json.contains("\"type\":\"rest\""));
        assert!(json.contains("\"restIdField\":\"itemId\""));
    }

    #[test]
    fn test_mock_deserialization_generates_uuid() {
        let json = r#"{
            "name": "Auto ID",
            "method": "GET",
            "path": "/test",
            "type": "static"
        }"#;
        let mock: Mock = serde_json::from_str(json).unwrap();
        assert!(!mock.id.is_empty());
        assert!(!mock.created_at.is_empty());
        assert!(!mock.updated_at.is_empty());
    }

    // --- PartialMock tests ---

    #[test]
    fn test_partial_mock_default() {
        let pm = PartialMock::default();
        assert!(pm.name.is_none());
        assert!(pm.method.is_none());
        assert!(pm.path.is_none());
        assert!(pm.mock_type.is_none());
    }

    #[test]
    fn test_partial_mock_serialization() {
        let pm = PartialMock {
            name: Some("Updated".to_string()),
            priority: Some(10),
            ..Default::default()
        };
        let json = serde_json::to_string(&pm).unwrap();
        assert!(json.contains("\"name\":\"Updated\""));
        assert!(json.contains("\"priority\":10"));
    }

    // --- FeedEntry tests ---

    #[test]
    fn test_feed_entry_serialization() {
        let entry = FeedEntry {
            id: "1".to_string(),
            timestamp: "2026-01-01T00:00:00.000Z".to_string(),
            method: "GET".to_string(),
            path: "/test".to_string(),
            matched_mock_id: Some("m1".to_string()),
            matched_mock_name: Some("Mock".to_string()),
            status: 200,
            duration_ms: 1.5,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"matchedMockId\":\"m1\""));
        assert!(json.contains("\"matchedMockName\":\"Mock\""));
        assert!(json.contains("\"durationMs\":1.5"));
    }

    // --- ConditionSource tests ---

    #[test]
    fn test_condition_source_serialization() {
        assert_eq!(
            serde_json::to_string(&ConditionSource::Header).unwrap(),
            "\"header\""
        );
        assert_eq!(
            serde_json::to_string(&ConditionSource::Query).unwrap(),
            "\"query\""
        );
        assert_eq!(
            serde_json::to_string(&ConditionSource::Body).unwrap(),
            "\"body\""
        );
    }

    // --- ConditionOp tests ---

    #[test]
    fn test_condition_op_serialization() {
        assert_eq!(
            serde_json::to_string(&ConditionOp::Exists).unwrap(),
            "\"exists\""
        );
        assert_eq!(
            serde_json::to_string(&ConditionOp::NotExists).unwrap(),
            "\"notExists\""
        );
        assert_eq!(
            serde_json::to_string(&ConditionOp::Equals).unwrap(),
            "\"equals\""
        );
        assert_eq!(
            serde_json::to_string(&ConditionOp::NotEquals).unwrap(),
            "\"notEquals\""
        );
    }

    // --- default_status function test ---

    #[test]
    fn test_default_status() {
        assert_eq!(default_status(), 200);
    }
}
