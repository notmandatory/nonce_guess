use axum::{http::StatusCode, response::IntoResponse, routing::get, Router};
use axum_login::permission_required;
use sqlx::SqlitePool;

use crate::web::auth::{Backend, Permission};

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/admin", get(self::get::admin))
        .route_layer(permission_required!(Backend, Permission::AssignAdmin))
}

mod get {
    use super::*;
    use crate::web::auth::AuthSession;

    pub async fn admin(auth_session: AuthSession) -> impl IntoResponse {
        match auth_session.user {
            Some(player) => AdminTemplate {
                player_name: &player.name,
            }
            .into_response(),

            None => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
