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

    // Static Mock
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<MockResponse>,

    // Conditional Mock
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Vec<Condition>>,
    #[serde(
        default,
        rename = "elseResponse",
        skip_serializing_if = "Option::is_none"
    )]
    pub else_response: Option<MockResponse>,

    // Sequence Mock
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<Vec<MockResponse>>,
    #[serde(
        default,
        rename = "sequenceMode",
        skip_serializing_if = "Option::is_none"
    )]
    pub sequence_mode: Option<SequenceMode>,

    // Script Mock
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    #[serde(
        default,
        rename = "scriptTimeoutMs",
        skip_serializing_if = "Option::is_none"
    )]
    pub script_timeout_ms: Option<u64>,

    // REST Mock
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
