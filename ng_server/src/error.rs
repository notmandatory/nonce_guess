use crate::IntoResponse;
use axum::http::StatusCode;
use axum::response::Response;
use axum::Json;
use ng_model::serde_json;
use ng_model::serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database: {0}")]
    Db(sqlx::Error),
    #[error("serde json: {0}")]
    SerdeJson(serde_json::Error),
    #[error("generic: {0}")]
    Generic(String),
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        Error::Db(err)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            Error::Db(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            Error::SerdeJson(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            Error::Generic(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
