use crate::app::AppState;
use crate::auth::backend::{AuthBackend, AuthSession};
use crate::auth::types::Permission;
use crate::guess::types::{Guess, GuessError, TargetError};
use askama_axum::IntoResponse;
use askama_axum::Template;
use axum::extract::State;
use axum::http::{HeaderValue, StatusCode};
use axum::routing::{get, post};
use axum::{Form, Router};
use axum_login::{login_required, permission_required};
use regex::Regex;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/target", post(target))
        .route_layer(permission_required!(AuthBackend, Permission::ChangeTarget))
        .route("/", get(home))
        .route("/target", get(home_target))
        .route("/", post(guess))
        .route("/guesses", get(home_guesses))
        .route_layer(login_required!(AuthBackend, login_url = "/login"))
}

// home page template
#[derive(Template)]
#[template(path = "home.html")]
pub struct HomeTemplate {
    target: Option<(u32, Option<u32>)>,
    change_target: bool,
    guesses: Vec<GuessTableData>,
    add_guess: bool,
}

#[derive(Template)]
#[template(path = "home_target.html")]
pub struct TargetTemplate {
    target: Option<(u32, Option<u32>)>,
}

#[derive(Template)]
#[template(path = "home_guesses.html")]
pub struct GuessesTemplate {
    guesses: Vec<GuessTableData>,
    add_guess: bool,
}

pub struct GuessTableData {
    pub name: String,
    pub hex: String,
    pub decimal: u32,
}

pub async fn home(
    auth_session: AuthSession,
    State(app_state): State<Arc<AppState>>,
) -> Result<HomeTemplate, GuessError> {
    let target = app_state.guess_backend.get_last_target_nonce().await?;
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

    let mut add_guess = false;
    let mut all_permissions = HashSet::new();

    if let Some(player) = auth_session.user {
        let permissions = auth_session.backend.get_player_permissions(&player).await?;
        all_permissions.extend(permissions);

        if let Some((height, None)) = target {
            add_guess = !app_state
                .guess_backend
                .any_guess(height, player.uuid)
                .await?;
        }
    }

    //let target = target.map(|(height, nonce)| (height, nonce.map(|nonce| format!("{:x}", nonce))));
    let change_target = all_permissions.contains(&Permission::ChangeTarget);
    Ok(HomeTemplate {
        target,
        change_target,
        guesses,
        add_guess,
    })
}

pub async fn home_target(
    auth_session: AuthSession,
    State(app_state): State<Arc<AppState>>,
) -> Result<TargetTemplate, GuessError> {
    let home_template = home(auth_session, State(app_state)).await?;
    Ok(TargetTemplate {
        target: home_template.target,
    })
}

pub async fn home_guesses(
    auth_session: AuthSession,
    State(app_state): State<Arc<AppState>>,
) -> Result<GuessesTemplate, GuessError> {
    let home_template = home(auth_session, State(app_state)).await?;
    Ok(GuessesTemplate {
        guesses: home_template.guesses,
        add_guess: home_template.add_guess,
    })
}

#[derive(Deserialize)]
pub struct GuessForm {
    guess: String,
}

pub async fn guess(
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
    let mut response = StatusCode::OK.into_response();
    response.headers_mut().insert(
        "HX-Location",
        HeaderValue::try_from("/").expect("home value"),
    );
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

pub async fn target(
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
    let mut response = StatusCode::OK.into_response();
    response.headers_mut().insert(
        "HX-Location",
        HeaderValue::try_from("/").expect("home value"),
    );
    Ok(response)
}

pub fn sort_guesses_by_target_diff(guesses: &mut [GuessTableData], target_nonce: u32) {
    guesses.sort_by(|a, b| {
        let target_a = target_nonce.abs_diff(a.decimal);
        let target_b = target_nonce.abs_diff(b.decimal);
        target_a.cmp(&target_b)
    })
}
