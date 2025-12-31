use super::backend::{AuthBackend, AuthSession};
use super::types::{datetime_now, LoginError, Player, RegisterError};
use crate::app::AppState;
use crate::types::InternalError;
use axum::extract::Query;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Form, Router};
use axum_login::login_required;
use axum_login::Error::Backend;
use password_auth::generate_hash;
use regex::Regex;
use rinja::Template;
use serde::Deserialize;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/profile", get(profile_page))
        .route("/profile", post(change_profile))
        .route("/logout", get(logout))
        .route_layer(login_required!(AuthBackend, login_url = "/login"))
        .route("/login", get(login_page))
        .route("/login", post(login_password))
        .route("/register", get(register_page))
        .route("/register", post(register_password))
}

/// login page template
#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    next: Option<String>,
}

#[axum::debug_handler]
async fn login_page(
    Query(NextUrl { next }): Query<NextUrl>,
) -> Result<impl IntoResponse, InternalError> {
    let page = LoginTemplate { next };
    let mut response = Html(page.render()?).into_response();
    response
        .headers_mut()
        .insert("HX-Refresh", HeaderValue::try_from("true").expect("value"));
    Ok(response)
}

#[derive(Template)]
#[template(path = "register.html")]
struct RegisterTemplate {}

#[axum::debug_handler]
async fn register_page() -> Result<impl IntoResponse, InternalError> {
    Ok(Html(RegisterTemplate {}.render()?))
}

#[derive(Template)]
#[template(path = "profile.html")]
struct ProfileTemplate {
    player: Player,
}

// Any filter defined in the module `filters` is accessible in your template.
pub mod filters {
    use chrono::{DateTime, Local, Utc};

    pub fn local_date(dt: &DateTime<Utc>, fmt: &str) -> rinja::Result<String> {
        let local_time = dt.with_timezone(&Local);
        Ok(local_time.format(fmt).to_string())
    }
}

#[axum::debug_handler]
async fn profile_page(auth_session: AuthSession) -> Result<impl IntoResponse, InternalError> {
    let player = auth_session.user.expect("player must be logged in");
    Ok(Html(ProfileTemplate { player }.render()?))
}

#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
    next: Option<String>,
}

impl LoginForm {
    pub fn password_hash(&self) -> String {
        generate_hash(&self.password)
    }

    pub fn credentials(&self) -> (String, String) {
        (self.username.clone(), self.password.clone())
    }
}

#[derive(Deserialize)]
pub struct RegisterForm {
    new_username: String,
    new_password: String,
    confirm_password: String,
    next: Option<String>,
}

impl RegisterForm {
    pub fn password_hash(&self) -> String {
        generate_hash(&self.new_password)
    }

    pub fn credentials(&self) -> (String, String) {
        (self.new_username.clone(), self.new_password.clone())
    }
}

// This allows us to extract the "next" field from the query string. We use this
// to redirect after log in.
#[derive(Debug, Deserialize)]
pub struct NextUrl {
    next: Option<String>,
}

async fn login_password(
    mut auth_session: AuthSession,
    Form(login_form): Form<LoginForm>,
) -> Result<Response, LoginError> {
    if let Some(player) = auth_session.authenticate(login_form.credentials()).await? {
        // update session so user is logged in
        auth_session.login(&player).await?;
        let mut response = StatusCode::OK.into_response();
        let next = login_form.next.unwrap_or("/".to_string());
        response.headers_mut().insert(
            "HX-Location",
            HeaderValue::try_from(next).expect("next value"),
        );
        Ok(response)
    } else {
        // failed authentication
        Err(LoginError::Authentication(login_form.username))
    }
}

async fn register_password(
    mut auth_session: AuthSession,
    Form(register_form): Form<RegisterForm>,
) -> Result<impl IntoResponse, RegisterError> {
    let new_username = register_form.new_username.clone();
    let new_password = register_form.new_password.clone();
    let confirm_password = register_form.confirm_password.clone();
    // validate username is unique
    if let Some(_player) = auth_session
        .backend
        .get_player_by_name(&new_username)
        .await
        .map_err(Backend)?
    {
        return Err(RegisterError::UserAlreadyRegistered(new_username));
    }
    // validate new credentials
    validate_name_password(&new_username, &new_password)?;
    if new_password != confirm_password {
        Err(RegisterError::UnconfirmedPassword)
    } else {
        let uuid = Uuid::new_v4();
        let password_hash = register_form.password_hash();
        let player = Player {
            uuid,
            name: new_username,
            password_hash,
            ..Default::default()
        };
        auth_session
            .backend
            .insert_player(&player)
            .await
            .map_err(Backend)?;
        if let Some(player) = auth_session
            .authenticate(register_form.credentials())
            .await?
        {
            // update session so user is logged in
            auth_session.login(&player).await?;
            let mut response = StatusCode::OK.into_response();
            let next = register_form.next.unwrap_or("/".to_string());
            response.headers_mut().insert(
                "HX-Location",
                HeaderValue::try_from(next).expect("next value"),
            );
            Ok(response)
        } else {
            // failed authentication
            Err(RegisterError::Authentication(register_form.new_username))
        }
    }
}

async fn change_profile(
    mut auth_session: AuthSession,
    Form(register_form): Form<RegisterForm>,
) -> Result<impl IntoResponse, RegisterError> {
    let orig_player = auth_session.user.clone().expect("player must be logged in");
    let new_username = register_form.new_username.clone();
    let new_password = register_form.new_password.clone();
    let new_confirm_password = register_form.confirm_password.clone();
    // validate username, if not the same, is unique
    if let Some(found_player) = auth_session
        .backend
        .get_player_by_name(&new_username)
        .await
        .map_err(Backend)?
    {
        if found_player.uuid != orig_player.uuid {
            return Err(RegisterError::UserAlreadyRegistered(new_username));
        }
    }
    // validate new credentials
    validate_name_password(&new_username, &new_password)?;
    if new_password != new_confirm_password {
        Err(RegisterError::UnconfirmedPassword)
    } else {
        let new_password_hash = register_form.password_hash();
        let new_player = Player {
            name: new_username,
            password_hash: new_password_hash,
            updated: datetime_now(),
            ..orig_player.clone()
        };
        auth_session
            .backend
            .change_player(&orig_player, &new_player)
            .await
            .map_err(Backend)?;
        // if the player record changed, re-authenticate
        if let Some(player) = auth_session
            .authenticate(register_form.credentials())
            .await?
        {
            // update session so user is logged in
            auth_session.login(&player).await?;
        }
        let mut response = StatusCode::OK.into_response();
        response.headers_mut().insert(
            "HX-Location",
            HeaderValue::try_from("/".to_string()).expect("next value"),
        );
        Ok(response)
    }
}

fn validate_name_password(new_username: &str, new_password: &str) -> Result<(), RegisterError> {
    let name_re = Regex::new(r#"[0-9a-zA-Z_]{3,20}"#).unwrap();
    let password_re = Regex::new(r#"[0-9a-zA-Z\d@$!%*?&#^_\.\-]{4,20}"#).unwrap();

    if !name_re.is_match(new_username) {
        Err(RegisterError::InvalidName)
    } else if !password_re.is_match(new_password) {
        Err(RegisterError::InvalidPassword)
    } else {
        Ok(())
    }
}

async fn logout(mut auth_session: AuthSession) -> impl IntoResponse {
    match auth_session.logout().await {
        Ok(_) => {
            let mut response = StatusCode::OK.into_response();
            response
                .headers_mut()
                .insert("HX-Refresh", HeaderValue::from_static("true"));
            response
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "failed to logout").into_response(),
    }
}

impl IntoResponse for RegisterError {
    fn into_response(self) -> Response {
        match self {
            RegisterError::InvalidName => {
                (
                    StatusCode::OK,
                    [("HX-Retarget", "#flash_message")],
                    "Name must be 3-20 characters and only include upper or lowercase A-Z, 0-9, and underscore.",
                )
                    .into_response()
            }
            RegisterError::InvalidPassword => {
                (
                    StatusCode::OK,
                    [("HX-Retarget", "#flash_message")],
                    "Password must be 4-20 characters and only include upper or lowercase A-Z, 0-9, and special characters [ @ $ ! % * ? & # ^ _ ].",
                )
                    .into_response()
            }
            RegisterError::UnconfirmedPassword => {
                (
                    StatusCode::OK,
                    [("HX-Retarget", "#flash_message")],
                    "Password and confirmation do not match.",
                )
                    .into_response()
            }
            RegisterError::UserAlreadyRegistered(user) => {
                info!("user already registered: {}", user);
                (
                    StatusCode::OK,
                    [("HX-Retarget", "#flash_message")],
                    "Name is already registered.",
                )
                    .into_response()
            }
            RegisterError::Authentication(name) => {
                info!("failed authentication for: {}", name);
                (
                    StatusCode::OK,
                    [("HX-Retarget", "#flash_message")],
                    "Failed authentication.",
                )
                    .into_response()
            }
            RegisterError::Internal(e) => {
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

impl IntoResponse for LoginError {
    fn into_response(self) -> Response {
        match self {
            LoginError::Authentication(name) => {
                info!("failed authentication for: {}", name);
                (
                    StatusCode::OK,
                    [("HX-Retarget", "#flash_message")],
                    format!("Failed authentication for: {}", name),
                )
                    .into_response()
            }
            LoginError::Internal(_) => {
                error!("{}", self);
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

#[cfg(test)]
mod test {
    use crate::auth::web::validate_name_password;

    #[test]
    fn test_validate_name_password() {
        assert!(validate_name_password("", "").is_err());
        assert!(validate_name_password("tester", "").is_err());
        assert!(validate_name_password("", "Test123$").is_err());
        assert!(validate_name_password("te", "Test123$").is_err());
        assert!(validate_name_password("tester", "Test1234").is_err());
        assert!(validate_name_password("tester", "Te1$").is_err());
        assert!(validate_name_password("tester", "Test123$").is_ok());
    }
}
