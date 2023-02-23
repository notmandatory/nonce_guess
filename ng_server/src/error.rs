use crate::IntoResponse;
use axum::http::StatusCode;
use axum::response::Response;
use axum::Json;
use ng_model::serde_json::json;
use std::num::ParseIntError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database: {0}")]
    Db(sqlx::Error),
    #[error("generic: {0}")]
    Generic(String),
    #[error("reqwest: {0}")]
    Reqwest(reqwest::Error),
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        Error::Db(err)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Reqwest(err)
    }
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
