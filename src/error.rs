use crate::IntoResponse;
use axum::http::StatusCode;
use axum::response::Response;
use axum::Json;
use serde_json::json;
use std::num::ParseIntError;

#[derive(Debug, thiserror::Error)]
pub enum Error<T = ()> {
    #[error("cbor ser error: {0}")]
    CborSer(#[from] ciborium::ser::Error<T>),
    #[error("cbor de error: {0}")]
    CborDe(#[from] ciborium::de::Error<T>),
    #[error("session error: {0}")]
    Session(#[from] tower_sessions::session::Error),
    #[error("request error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("parse guess error: {0}")]
    ParseGuess(#[from] ParseIntError),
    #[error("invalid permission error: {0}")]
    InvalidPermission(String),
    #[error("invalid role error: {0}")]
    InvalidRole(String),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            Error::CborSer(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            Error::CborDe(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            Error::Session(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            Error::Reqwest(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            Error::ParseGuess(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            Error::InvalidPermission(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            Error::InvalidRole(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
