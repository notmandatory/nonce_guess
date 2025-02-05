use super::types::{Guess, GuessError, TargetError};
use crate::app::AppState;
use crate::auth::backend::{AuthBackend, AuthSession};
use crate::auth::types::Permission;
use crate::types::InternalError;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Form, Router};
use axum_login::{login_required, permission_required};
use regex::Regex;
use rinja::Template;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/target", post(target_form))
        .route_layer(permission_required!(AuthBackend, Permission::ChangeTarget))
        .route("/", get(home_page))
        .route("/target", get(target_page))
        .route("/target/table", get(target_table))
        .route("/", post(guess_form))
        .route("/guess/table", get(guess_table))
        .route_layer(login_required!(AuthBackend, login_url = "/login"))
}

// home page template
#[derive(Template)]
#[template(path = "home.html")]
pub struct HomeTemplate {
    target: Option<(u32, Option<u32>)>,
    guesses: Vec<GuessTableData>,
    add_guess: bool,
}

#[derive(Template)]
#[template(path = "target.html")]
pub struct TargetTemplate {
    target: Option<(u32, Option<u32>)>,
    change_target: bool,
}

#[derive(Template)]
#[template(path = "target_table.html")]
pub struct TargetTable {
    target: Option<(u32, Option<u32>)>,
}

#[derive(Template)]
#[template(path = "guess_table.html")]
pub struct GuessTable {
    guesses: Vec<GuessTableData>,
    add_guess: bool,
}

pub struct GuessTableData {
    pub name: String,
    pub hex: String,
    pub decimal: u32,
}

pub async fn home_page(
    auth_session: AuthSession,
    State(app_state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, GuessError> {
    let target = app_state.guess_backend.get_last_target_nonce().await?;
    // let mut guesses = if let Some((height, _nonce)) = target {
    //     let guesses = app_state.guess_backend.target_guesses(height).await?;
    //     let players = auth_session
    //         .backend
    //         .get_players()
    //         .await?
    //         .into_iter()
    //         .map(|player| (player.uuid, player.name))
    //         .collect::<HashMap<Uuid, String>>();
    //     guesses
    //         .into_iter()
    //         .map(|guess| {
    //             let player_name = players.get(&guess.player).expect("player name").clone();
    //             let nonce_hex = format!("{:x}", guess.nonce);
    //             let nonce_decimal = guess.nonce;
    //             GuessTableData {
    //                 name: player_name,
    //                 hex: nonce_hex,
    //                 decimal: nonce_decimal,
    //             }
    //         })
    //         .collect::<Vec<GuessTableData>>()
    // } else {
    //     Vec::<GuessTableData>::new()
    // };

    // if let Some((_height, Some(nonce))) = target {
    //     // sort by abs distance to target nonce
    //     sort_guesses_by_target_diff(&mut guesses, nonce);
    // } else {
    //     // sort by player name
    //     guesses.sort_by(|a, b| a.name.cmp(&b.name));
    // }
    let guesses = guesses(&auth_session, app_state.clone(), target).await?;
    let add_guess = add_guess(&auth_session, app_state, target).await?;

    Ok(Html(
        HomeTemplate {
            target,
            guesses,
            add_guess,
        }
        .render()
        .map_err(InternalError::from)?,
    ))
}

async fn guesses(
    auth_session: &AuthSession,
    app_state: Arc<AppState>,
    target: Option<(u32, Option<u32>)>,
) -> Result<Vec<GuessTableData>, GuessError> {
    let mut guesses = if let Some((height, _nonce)) = target {
        let guesses = app_state.guess_backend.target_guesses(height).await?;
        let players = auth_session
            .backend
            .get_players()
            .await?
            .into_iter()
            .map(|player| (player.uuid, player.name))
            .collect::<HashMap<Uuid, String>>();
        guesses
            .into_iter()
            .map(|guess| {
                let player_name = players.get(&guess.player).expect("player name").clone();
                let nonce_hex = format!("{:x}", guess.nonce);
                let nonce_decimal = guess.nonce;
                GuessTableData {
                    name: player_name,
                    hex: nonce_hex,
                    decimal: nonce_decimal,
                }
            })
            .collect::<Vec<GuessTableData>>()
    } else {
        Vec::<GuessTableData>::new()
    };
    if let Some((_height, Some(nonce))) = target {
        // sort by abs distance to target nonce
        sort_guesses_by_target_diff(&mut guesses, nonce);
    } else {
        // sort by player name
        guesses.sort_by(|a, b| a.name.cmp(&b.name));
    }
    Ok(guesses)
}

async fn add_guess(
    auth_session: &AuthSession,
    app_state: Arc<AppState>,
    target: Option<(u32, Option<u32>)>,
) -> Result<bool, GuessError> {
    let mut add_guess = false;
    if let Some(player) = &auth_session.user {
        if let Some((height, None)) = target {
            add_guess = !app_state
                .guess_backend
                .any_guess(height, player.uuid)
                .await?;
        }
    }
    Ok(add_guess)
}

pub async fn target_page(
    auth_session: AuthSession,
    State(app_state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, GuessError> {
    let target = app_state.guess_backend.get_last_target_nonce().await?;

    let mut change_target = false;
    if let Some(player) = auth_session.user {
        let permissions = auth_session.backend.get_player_permissions(&player).await?;
        change_target = permissions.contains(&Permission::ChangeTarget)
    }

    Ok(Html(
        TargetTemplate {
            target,
            change_target,
        }
        .render()
        .map_err(InternalError::from)?,
    ))
}

pub async fn target_table(
    State(app_state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, GuessError> {
    let target = app_state.guess_backend.get_last_target_nonce().await?;
    Ok(Html(
        TargetTable { target }
            .render()
            .map_err(InternalError::from)?,
    ))
}

pub async fn guess_table(
    auth_session: AuthSession,
    State(app_state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, GuessError> {
    let target = app_state.guess_backend.get_last_target_nonce().await?;
    let guesses = guesses(&auth_session, app_state.clone(), target).await?;
    let add_guess = add_guess(&auth_session, app_state, target).await?;

    Ok(Html(
        GuessTable { guesses, add_guess }
            .render()
            .map_err(InternalError::from)?,
    ))
}

#[derive(Deserialize)]
pub struct GuessForm {
    guess: String,
}

pub async fn guess_form(
    auth_session: AuthSession,
    State(app_state): State<Arc<AppState>>,
    Form(guess_form): Form<GuessForm>,
) -> Result<impl IntoResponse, GuessError> {
    let guess = guess_form.guess.clone();
    if let Some((height, nonce_opt)) = app_state
        .guess_backend
        .get_last_target_nonce()
        .await
        .map_err(Into::<GuessError>::into)?
    {
        if let Some(nonce) = nonce_opt {
            return Err(GuessError::ConfirmedTarget(nonce));
        }
        let nonce = validate_guess(guess)?;
        if let Some(player) = auth_session.user {
            let guess = Guess {
                player: player.uuid,
                nonce,
            };
            app_state.guess_backend.insert_guess(height, guess).await?;
        }
    }
    let response = StatusCode::OK.into_response();
    Ok(response)
}

fn validate_guess(nonce: String) -> Result<u32, GuessError> {
    let nonce_re = Regex::new(r#"[0-9a-fA-F]"#).unwrap();

    if !nonce_re.is_match(nonce.as_str()) || nonce.len() > 8 {
        Err(GuessError::InvalidNonce(nonce))
    } else {
        let nonce_u32 =
            u32::from_str_radix(nonce.as_str(), 16).map_err(|_| GuessError::InvalidNonce(nonce))?;
        Ok(nonce_u32)
    }
}

#[derive(Deserialize)]
pub struct TargetForm {
    height: u32,
}

pub async fn target_form(
    State(app_state): State<Arc<AppState>>,
    Form(target_form): Form<TargetForm>,
) -> Result<impl IntoResponse, TargetError> {
    let current_target = app_state
        .guess_backend
        .get_last_target_nonce()
        .await
        .map_err(Into::<TargetError>::into)?;
    match current_target {
        None => {
            app_state
                .guess_backend
                .insert_target(target_form.height, None)
                .await
                .map_err(Into::<TargetError>::into)?;
            info!("Created new target at height {}", target_form.height);
        }
        Some((height, Some(_nonce))) if target_form.height > height => {
            app_state
                .guess_backend
                .insert_target(target_form.height, None)
                .await
                .map_err(Into::<TargetError>::into)?;
            info!("Created new target at height {}", target_form.height);
        }
        Some((_height, Some(_nonce))) => {
            // new height is less than or equal to current height
            return Err(TargetError::InvalidHeight(target_form.height));
        }
        Some((height, None)) if target_form.height > height => {
            // block nonce for current height no yet set
            // replace current target with new target height
            app_state
                .guess_backend
                .replace_target(height, target_form.height)
                .await
                .map_err(Into::<TargetError>::into)?;
            info!(
                "Replaced target at height {} with new target at height {}",
                height, target_form.height
            );
        }
        Some((_height, None)) => {
            // new height is less than or equal to current height
            return Err(TargetError::InvalidHeight(target_form.height));
        }
    }
    let response = StatusCode::OK.into_response();
    Ok(response)
}

pub fn sort_guesses_by_target_diff(guesses: &mut [GuessTableData], target_nonce: u32) {
    guesses.sort_by(|a, b| {
        let target_a = target_nonce.abs_diff(a.decimal);
        let target_b = target_nonce.abs_diff(b.decimal);
        target_a.cmp(&target_b)
    })
}

impl IntoResponse for GuessError {
    fn into_response(self) -> Response {
        match self {
            GuessError::DuplicateGuess(height) => {
                info!("player already made a guess for target height: {}", height);
                (
                    StatusCode::OK,
                    [("HX-Retarget", "#flash_message")],
                    format!("You already made a guess for target height: {}", height),
                )
                    .into_response()
            }
            GuessError::InvalidNonce(_) => (
                StatusCode::OK,
                [("HX-Retarget", "#flash_message")],
                "Invalid nonce.",
            )
                .into_response(),
            GuessError::DuplicateNonce(_) => (
                StatusCode::OK,
                [("HX-Retarget", "#flash_message")],
                "Guess made by another player.",
            )
                .into_response(),
            GuessError::ConfirmedTarget(height) => {
                info!("block was already confirmed for target height: {}", height);
                (
                    StatusCode::OK,
                    [("HX-Retarget", "#flash_message")],
                    "Block already confirmed.",
                )
                    .into_response()
            }
            GuessError::MissingTarget(height) => {
                info!("target does not exist for height: {}", height);
                (
                    StatusCode::OK,
                    [("HX-Retarget", "#flash_message")],
                    format!("No target for height: {}", height),
                )
                    .into_response()
            }
            GuessError::Internal(e) => {
                error!("{}", e);
                (
                    StatusCode::OK,
                    [("HX-Retarget", "#flash_message")],
                    "Internal server error.",
                )
                    .into_response()
            }
        }
    }
}

impl IntoResponse for TargetError {
    fn into_response(self) -> Response {
        match self {
            TargetError::InvalidHeight(height) => {
                info!(
                    "new height less than or equal to current target height: {}",
                    height
                );
                (
                    StatusCode::OK,
                    [("HX-Retarget", "#flash_message")],
                    "New height must be greater than current target height.",
                )
                    .into_response()
            }
            TargetError::Internal(e) => {
                error!("{}", e);
                (
                    StatusCode::OK,
                    [("HX-Retarget", "#flash_message")],
                    "Internal server error.",
                )
                    .into_response()
            }
        }
    }
}
