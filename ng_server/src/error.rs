use crate::IntoResponse;
use axum::http::StatusCode;
use axum::response::Response;
use axum::Json;
use serde_json::json;
use std::num::ParseIntError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database: {0}")]
    Db(#[from] sqlx::Error),
    #[error("generic: {0}")]
    Generic(String),
    #[error("reqwest: {0}")]
    Reqwest(#[from] reqwest::Error),
}

impl From<ParseIntError> for Error {
    fn from(err: ParseIntError) -> Self {
        Error::Generic(err.to_string())
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            Error::Db(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            Error::Generic(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
            Error::Reqwest(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum WebauthnError {
    #[error("unknown webauthn error")]
    Unknown,
    #[error("Corrupt Session")]
    CorruptSession,
    #[error("User Not Found")]
    UserNotFound,
    #[error("User Has No Credentials")]
    UserHasNoCredentials,
    #[error("Deserialising Session failed: {0}")]
    InvalidSessionState(#[from] tower_sessions::session::Error),
}
impl IntoResponse for WebauthnError {
    fn into_response(self) -> Response {
        let body = match self {
            WebauthnError::CorruptSession => "Corrupt Session",
            WebauthnError::UserNotFound => "User Not Found",
            WebauthnError::Unknown => "Unknown Error",
            WebauthnError::UserHasNoCredentials => "User Has No Credentials",
            WebauthnError::InvalidSessionState(_) => "Deserialising Session failed",
        };

        // its often easiest to implement `IntoResponse` by calling other implementations
        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}
