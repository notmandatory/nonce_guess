use askama::Template;
use axum::{http::StatusCode, response::IntoResponse, routing::get, Router};

#[derive(Template)]
#[template(path = "pages/admin.html")]
struct AdminTemplate<'a> {
    player_name: &'a str,
}

pub fn router() -> Router<()> {
    Router::new().route("/admin", get(self::get::admin))
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
