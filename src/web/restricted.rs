use super::auth::{types::Permission, Backend};
use axum::{routing::post, Router};
use axum_login::permission_required;
use redb::Database;
use std::sync::Arc;

pub fn router() -> Router<Arc<Database>> {
    Router::new()
        .route("/target", post(self::post::target))
        .route_layer(permission_required!(Backend, Permission::ChangeTargetBlock))
}

mod post {
    use axum::http::StatusCode;
    use axum::{extract::State, response::IntoResponse, Form};
    use axum_login::AuthzBackend;
    use log::info;
    use redb::Database;
    use serde::Deserialize;
    use std::sync::Arc;

    use crate::{error::Error, web::auth::AuthSession};

    #[derive(Deserialize)]
    pub struct TargetForm {
        block: u32,
    }

    pub async fn target(
        auth_session: AuthSession,
        State(db): State<Arc<Database>>,
        Form(target_form): Form<TargetForm>,
    ) -> Result<impl IntoResponse, Error> {
        let new_block = target_form.block;
        dbg!(&new_block);
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
        //         //let _player_name = tx.select_player_name(&uuid).await.ok();
        //         //let current_target = tx.select_current_target().await.ok();
        //         //let current_block = current_target.as_ref().map(|t| t.block);
        //
        //         // update target block
        //         tx.update_target(new_block)
        //             .await
        //             .map_err(|e| Error::Db(e))?;
        //         tx.set_guesses_block(new_block)
        //             .await
        //             .map_err(|e| Error::Db(e))?;
        //         tx.commit().await.expect("commit");
        //
        //         let mut tx = pool.begin().await.expect("tx");
        //         let target = tx.select_current_target().await.ok();
        //         let guesses = if let Some(some_target) = &target {
        //             tx.select_block_guesses(some_target.block).await.ok()
        //         } else {
        //             tx.select_guesses().await.ok()
        //         }
        //         .unwrap_or_default();
        //
        //         let player_guess =
        //             guesses
        //                 .iter()
        //                 .find_map(|g| if g.uuid == uuid { Some(g.nonce) } else { None });
        //
        //         Ok(
        //             template::home::home_page(target, change_target, player_guess, guesses)
        //                 .into_response(),
        //         )
        //     }
        //     None => Err(Error::Auth(auth::Error::Unknown)),
        // }
        Ok(StatusCode::OK.into_response())
    }
}
