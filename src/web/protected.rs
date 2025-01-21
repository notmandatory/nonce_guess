use askama_axum::Template;
use axum::routing::post;
use axum::{http::StatusCode, routing::get, Router};
use redb::Database;
use std::sync::Arc;

pub fn router() -> Router<Arc<Database>> {
    Router::new()
        .route("/", get(self::get::home))
        .route("/", post(self::post::guess))
}

/// home page template
#[derive(Template)]
#[template(path = "home.html")]
pub struct HomeTemplate {}

mod get {
    use super::super::auth::{types::Permission, AuthSession};
    use super::*;
    use crate::model::{sort_guesses_by_target_diff, Target};
    use axum::extract::State;
    use axum::response::IntoResponse;
    use axum_login::AuthzBackend;
    use tracing::debug;

    pub async fn home(
        auth_session: AuthSession,
        State(db): State<Arc<Database>>,
    ) -> impl IntoResponse {
        debug!("auth_session: {:?}", &auth_session);
        // match auth_session.user {
        //     Some(player) => {
        //         let change_target = auth_session
        //             .backend
        //             .has_perm(&player, Permission::ChangeTargetBlock)
        //             .await
        //             .unwrap(); // TODO map error
        //         let uuid = player.uuid;
        //         let mut tx = pool.begin().await.expect("tx");
        //         let target = tx.select_current_target().await.ok();
        //
        //         let guesses = match &target {
        //             Some(Target {
        //                 block: _,
        //                 nonce: None,
        //             }) => tx.select_guesses().await.ok().unwrap_or_default(),
        //             Some(Target {
        //                 block,
        //                 nonce: Some(nonce),
        //             }) => {
        //                 let mut guesses = tx
        //                     .select_block_guesses(*block)
        //                     .await
        //                     .ok()
        //                     .unwrap_or_default();
        //                 sort_guesses_by_target_diff(&mut guesses, *nonce);
        //                 guesses
        //             }
        //             None => tx.select_guesses().await.ok().unwrap_or_default(),
        //         };
        //         let my_guess: Option<u32> = guesses
        //             .iter()
        //             .find(|g| g.uuid == uuid)
        //             .map(|guess| guess.nonce);
        //
        //         home_page(target, change_target, my_guess, guesses).into_response()
        //     }
        //     None => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        // }
        HomeTemplate {}
    }
}

mod post {
    use crate::error::Error;
    use crate::web::auth::AuthSession;
    use axum::extract::State;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::Form;
    use axum_login::AuthzBackend;
    use redb::Database;
    use serde::Deserialize;
    use std::sync::Arc;
    use tracing::info;

    #[derive(Deserialize)]
    pub struct GuessForm {
        guess: String,
    }

    pub async fn guess(
        auth_session: AuthSession,
        State(pool): State<Arc<Database>>,
        Form(guess_form): Form<GuessForm>,
    ) -> Result<impl IntoResponse, Error> {
        let guess = guess_form.guess;
        dbg!(&guess);
        // match auth_session.user {
        //     Some(player) => {
        //         info!("Current player: {:?}", player);
        //         let change_target = auth_session
        //             .backend
        //             .has_perm(&player, Permission::ChangeTargetBlock)
        //             .await
        //             .unwrap();
        //
        //         let uuid = player.uuid;
        //         let mut tx = pool.begin().await.expect("tx");
        //         let _player_name = tx.select_player_name(&uuid).await.ok();
        //         let target = tx.select_current_target().await.ok();
        //
        //         // add guess
        //         let nonce = u32::from_str_radix(guess.as_str(), 16)?;
        //         tx.insert_guess(&uuid, nonce)
        //             .await
        //             .map_err(|e| Error::Db(e))?;
        //         tx.commit().await.expect("commit");
        //
        //         let mut tx = pool.begin().await.expect("tx");
        //         let guesses = match &target {
        //             Some(Target {
        //                 block: _,
        //                 nonce: None,
        //             }) => tx.select_guesses().await.ok().unwrap_or_default(),
        //             Some(Target {
        //                 block,
        //                 nonce: Some(nonce),
        //             }) => {
        //                 let mut guesses = tx
        //                     .select_block_guesses(*block)
        //                     .await
        //                     .ok()
        //                     .unwrap_or_default();
        //                 sort_guesses_by_target_diff(&mut guesses, *nonce);
        //                 guesses
        //             }
        //             None => tx.select_guesses().await.ok().unwrap_or_default(),
        //         };
        //
        //         Ok(
        //             template::home::home_page(target, change_target, Some(nonce), guesses)
        //                 .into_response(),
        //         )
        //     }
        //     None => Err(Error::Auth(auth::Error::Unknown)),
        // }
        Ok(StatusCode::OK.into_response())
    }
}
