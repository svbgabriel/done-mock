use thiserror::Error;

/// Custom error types for the mock server.
///
/// Replaces `anyhow::Error` in the `MockStore` trait to enable
/// specific error handling in admin handlers.
#[derive(Debug, Error)]
pub enum MockServerError {
    /// The requested mock was not found.
    #[error("Mock with id '{0}' not found")]
    NotFound(String),

    /// A database or storage operation failed.
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// Input validation failed.
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// A script execution failed.
    #[error("Script error: {0}")]
    ScriptError(String),
}

impl MockServerError {
    /// Returns the appropriate HTTP status code for this error.
    pub fn status_code(&self) -> axum::http::StatusCode {
        match self {
            MockServerError::NotFound(_) => axum::http::StatusCode::NOT_FOUND,
            MockServerError::ValidationError(_) => axum::http::StatusCode::BAD_REQUEST,
            MockServerError::DatabaseError(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            MockServerError::ScriptError(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Returns a short error type label for API responses.
    pub fn error_type(&self) -> &'static str {
        match self {
            MockServerError::NotFound(_) => "Not Found",
            MockServerError::ValidationError(_) => "Validation Error",
            MockServerError::DatabaseError(_) => "Internal Server Error",
            MockServerError::ScriptError(_) => "Script Error",
        }
    }
}

#[cfg(feature = "postgres")]
impl From<sqlx::Error> for MockServerError {
    fn from(err: sqlx::Error) -> Self {
        MockServerError::DatabaseError(err.to_string())
    }
}

impl From<serde_json::Error> for MockServerError {
    fn from(err: serde_json::Error) -> Self {
        MockServerError::DatabaseError(err.to_string())
    }
}
