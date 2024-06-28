use crate::model::{Guess, Target};
use askama::Template;
use axum::{http::StatusCode, routing::get, Router};
use axum::routing::post;
use sqlx::SqlitePool;
use crate::error::Error;

#[derive(Template)]
#[template(path = "pages/home.html")]
struct HomeTemplate {
    player_name: String,
    target: Option<Target>,
    my_guess: Option<String>,
    guesses: Vec<Guess>,
}

#[derive(Template)]
#[template(path = "fragments/guesses.html")]
struct GuessesFragment {
    guesses: Vec<Guess>,
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(self::get::home))
        .route("/", post(self::post::guess))
}

mod get {
    use std::sync::Arc;
    use axum::extract::State;
    use super::*;
    use crate::web::auth::AuthSession;
    use axum::response::IntoResponse;
    use sqlx::sqlite::SqlitePool;
    use tracing::debug;
    use crate::db::Db;
    use crate::error::Error;
    use crate::model::Block;

    pub async fn home(auth_session: AuthSession, State(pool): State<SqlitePool>) -> impl IntoResponse {
        debug!("auth_session: {:?}", &auth_session);
        match auth_session.user {
            Some(player) => {
                let player_name = player.name;
                let uuid = player.uuid;
                let mut tx = pool.begin().await.expect("tx");
                let target = tx.select_current_target().await.ok();
                let guesses = if let Some(some_target) = &target {
                    tx.select_block_guesses(some_target.block).await.ok()
                } else {
                    tx.select_guesses().await.ok()
                }.unwrap_or_default();
                let my_guess: Option<String> = guesses
                    .iter()
                    .find(|g| g.uuid == uuid)
                    .map(|guess| guess.clone().name);

                HomeTemplate {
                    player_name,
                    target,
                    my_guess,
                    guesses,
                }.into_response()
            }
            None => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }

    async fn get_target_nonce(State(pool): State<SqlitePool>) -> Result<String, Error> {
        //let nonce = u32::from_str(nonce.as_str())?;
        let client = reqwest::Client::new();
        let mut tx = pool.begin().await.map_err(crate::db::Error::Sqlx)?;
        let current_target = tx.select_current_target().await?;
        let block_height_response = client
            .get(format!(
                "https://mempool.space/api/block-height/{}",
                current_target.block
            ))
            .send()
            .await?;
        if block_height_response.status().is_success() {
            let block_hash = block_height_response.text().await?;
            let block_response = client
                .get(format!("https://mempool.space/api/block/{}", block_hash))
                .send()
                .await?;
            if block_response.status().is_success() {
                let block: Block = block_response.json().await?;
                let nonce = block.nonce;
                tx.set_current_nonce(nonce).await?;
                tx.set_guesses_block(block.height).await?;
                tx.commit().await.map_err(crate::db::Error::Sqlx)?;
                return Ok(nonce.to_string());
            }
        }
        Ok(String::default())
    }
}

mod post {
    use std::sync::Arc;
    use axum::extract::State;
    use axum::Form;
    use super::*;
    use crate::web::auth::AuthSession;
    use axum::response::IntoResponse;
    use serde::Deserialize;
    use sqlx::sqlite::SqlitePool;
    use tracing::{debug, info};
    use webauthn_rs::prelude::WebauthnError::UserNotPresent;
    use crate::db::Db;
    use crate::error::Error;
    use crate::model::Block;
    use crate::web::auth;

    #[derive(Deserialize)]
    pub struct GuessForm {
        guess: String,
    }

    pub async fn guess(
        auth_session: AuthSession,
        State(pool): State<SqlitePool>,
        Form(guess_form): Form<GuessForm>,
    ) -> Result<impl IntoResponse, Error> {
        let guess = guess_form.guess;
        dbg!(&guess);
        match auth_session.user {
            Some(player) => {
                info!("Current player: {:?}", player);

                let uuid = player.uuid;
                let mut tx = pool.begin().await.expect("tx");
                let player_name = tx.select_player_name(&uuid).await.ok();
                let target = tx.select_current_target().await.ok();

                // add guess
                let block = target.as_ref().map(|t| t.block);
                let nonce = u32::from_str_radix(guess.as_str(), 16)?;
                tx.insert_guess(&uuid, block, nonce)
                    .await
                    .map_err(|e| Error::Db(e))?;
                tx.commit().await.expect("commit");

                let mut tx = pool.begin().await.expect("tx");
                let guesses = if let Some(some_target) = &target {
                    tx.select_block_guesses(some_target.block).await.ok()
                } else {
                    tx.select_guesses().await.ok()
                }
                    .unwrap_or_default();

                let my_guess = Some(Guess {
                    uuid,
                    block,
                    name: player_name.clone().unwrap(),
                    nonce,
                });

                Ok(HomeTemplate {
                    player_name: player_name.unwrap(),
                    target,
                    my_guess: Some(guess),
                    guesses,
                })
            }
            None => Err(Error::Auth(auth::Error::Unknown)),
        }
    }
}
