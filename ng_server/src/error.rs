use crate::IntoResponse;
use axum::http::StatusCode;
use axum::response::Response;
use axum::Json;
use serde_json::json;
use std::num::ParseIntError;

#[derive(Debug, thiserror::Error)]
pub enum Error<T = ()> {
    #[error("database: {0}")]
    Db(#[from] sqlx::Error),
    #[error("CborSer: {0}")]
    CborSer(#[from] ciborium::ser::Error<T>),
    #[error("CborDe: {0}")]
    CborDe(#[from] ciborium::de::Error<T>),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            Error::Db(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            Error::CborSer(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            Error::CborDe(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
