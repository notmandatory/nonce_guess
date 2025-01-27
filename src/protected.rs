use crate::app::AppState;
use crate::auth::backend::AuthBackend;
use crate::guess::backend::GuessBackend;
use crate::guess::types::Guess;
use askama_axum::Template;
use axum::routing::post;
use axum::{http::StatusCode, routing::get, Router};
use axum_login::login_required;
use redb::Database;
use std::sync::Arc;

// pub fn router() -> Router<Arc<AppState>> {
//     Router::new()
//         .route("/", get(self::get::home))
//         .route("/", post(self::post::guess))
//         .route_layer(login_required!(AuthBackend, login_url = "/login"))
// }

// /// home page template
// #[derive(Template)]
// #[template(path = "home.html")]
// pub struct HomeTemplate {
//     target: Option<(u32, Option<String>)>,
//     change_target: bool,
//     guesses: Vec<Guess>,
//     add_guess: bool,
// }

// mod get {
//     use super::*;
//     use crate::auth::{backend::AuthSession, types::Permission};
//     use crate::guess::backend::GuessBackend;
//     use crate::guess::types::GuessError;
//     use axum::extract::State;
//     use axum::response::IntoResponse;
//     use axum_login::AuthzBackend;
//     use std::collections::HashSet;
//     use tracing::debug;
//
//     pub async fn home(
//         auth_session: AuthSession,
//         State(app_state): State<Arc<AppState>>,
//     ) -> Result<HomeTemplate, GuessError> {
//         debug!("auth_session: {:?}", &auth_session);
//         // match auth_session.user {
//         //     Some(player) => {
//         //         let change_target = auth_session
//         //             .backend
//         //             .has_perm(&player, Permission::ChangeTargetBlock)
//         //             .await
//         //             .unwrap(); // TODO map error
//         //         let uuid = player.uuid;
//         //         let mut tx = pool.begin().await.expect("tx");
//         //         let target = tx.select_current_target().await.ok();
//         //
//         //         let guesses = match &target {
//         //             Some(Target {
//         //                 block: _,
//         //                 nonce: None,
//         //             }) => tx.select_guesses().await.ok().unwrap_or_default(),
//         //             Some(Target {
//         //                 block,
//         //                 nonce: Some(nonce),
//         //             }) => {
//         //                 let mut guesses = tx
//         //                     .select_block_guesses(*block)
//         //                     .await
//         //                     .ok()
//         //                     .unwrap_or_default();
//         //                 sort_guesses_by_target_diff(&mut guesses, *nonce);
//         //                 guesses
//         //             }
//         //             None => tx.select_guesses().await.ok().unwrap_or_default(),
//         //         };
//         //         let my_guess: Option<u32> = guesses
//         //             .iter()
//         //             .find(|g| g.uuid == uuid)
//         //             .map(|guess| guess.nonce);
//         //
//         //         home_page(target, change_target, my_guess, guesses).into_response()
//         //     }
//         //     None => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
//         // }
//         let target = app_state
//             .guess_backend
//             .get_last_target()
//             .await?;
//
//         let mut guesses = Vec::new();
//         let mut add_guess = false;
//         let mut all_permissions = HashSet::new();
//
//         if let Some(player) = auth_session.user {
//             let permissions = auth_session.backend.get_player_permissions(&player).await?;
//             all_permissions.extend(permissions);
//
//             if let Some((height, None)) = target {
//                 add_guess = !app_state.guess_backend.any_guess(height, player.uuid).await?;
//             }
//         }
//
//         let target = target.map(|(height, nonce)| (height, nonce.map(|nonce| format!("{:x}", nonce))));
//         let change_target = all_permissions.contains(&Permission::ChangeTarget);
//         Ok(HomeTemplate {
//             target,
//             change_target,
//             guesses,
//             add_guess,
//         })
//     }
// }

mod post {
    use crate::app::AppState;
    use crate::auth::backend::AuthSession;
    use axum::extract::State;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::Form;
    use serde::Deserialize;
    use std::sync::Arc;

    #[derive(Deserialize)]
    pub struct GuessForm {
        guess: String,
    }

    pub async fn guess(
        auth_session: AuthSession,
        State(app_state): State<Arc<AppState>>,
        Form(guess_form): Form<GuessForm>,
    ) -> Result<impl IntoResponse, ()> {
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
