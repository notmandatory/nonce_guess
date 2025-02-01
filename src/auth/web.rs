use super::backend::AuthSession;
use super::types::{LoginError, Player, RegisterError};
use crate::app::AppState;
use askama_axum::IntoResponse;
use askama_axum::Template;
use axum::extract::Query;
use axum::http::{HeaderValue, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Form, Router};
use axum_login::Error::Backend;
use password_auth::generate_hash;
use regex::{Regex, RegexSet};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/login", get(login_page))
        .route("/login", post(login_password))
        .route("/register", get(register_page))
        .route("/register", post(register_password))
        .route("/logout", get(logout))
}

/// login page template
#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    next: Option<String>,
}

#[axum::debug_handler]
async fn login_page(Query(NextUrl { next }): Query<NextUrl>) -> Response {
    let page = LoginTemplate { next };
    let mut response = page.into_response();
    response
        .headers_mut()
        .insert("HX-Refresh", HeaderValue::try_from("true").expect("value"));
    response
}

#[derive(Template)]
#[template(path = "register.html")]
struct RegisterTemplate {}

#[axum::debug_handler]
async fn register_page() -> RegisterTemplate {
    RegisterTemplate {}
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
    // validate credentials
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

fn validate_name_password(new_username: &str, new_password: &str) -> Result<(), RegisterError> {
    let name_re = Regex::new(r#"[0-9a-zA-Z_]{3,20}"#).unwrap();
    let password_re = RegexSet::new([
        r#".*[a-z]"#,
        r#".*[A-Z]"#,
        r#".*\d"#,
        r#".*[@$!%*?&#^_\.\-]"#,
        r#"[A-Za-z\d@$!%*?&#^_\.\-]{8,20}"#,
    ])
    .unwrap();

    if !name_re.is_match(new_username) {
        Err(RegisterError::InvalidName)
    } else if !password_re.matches(new_password).matched_all() {
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
