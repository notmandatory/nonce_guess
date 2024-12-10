use super::template::login::login_page;
use axum::http::HeaderValue;
use axum::routing::{get, post};
use axum::{
    extract::{Json, Path},
    http::StatusCode,
    response::IntoResponse,
    Router,
};
use axum_login::axum::async_trait;
use axum_login::{AuthUser, AuthnBackend, AuthzBackend, UserId};
use maud::Markup;
use sqlx::{Pool, Sqlite, SqlitePool};
use std::collections::HashSet;
use std::sync::Arc;
use tower_sessions::Session;
use tracing::{debug, error, info};
use uuid::Uuid;

/*
 * Webauthn RS auth handlers.
 * These files use webauthn to process the data received from each route, and are closely tied to axum
 */

// 1. Import the prelude - this contains everything needed for the server to function.
use crate::db;
use crate::db::Db;
use crate::model::Player;
use crate::web::auth::Error::UserAlreadyRegistered;
use webauthn_rs::prelude::*;

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/login", get(login))
        .route("/register_start/:username", post(start_register))
        .route("/register_finish", post(finish_register))
        .route("/login_start/:username", post(start_authentication))
        .route("/login_finish", post(finish_authentication))
        .route("/logout", get(logout))
}

pub async fn login() -> Markup {
    login_page()
}

// 2. The first step a client (user) will carry out is requesting a credential to be
// registered. We need to provide a challenge for this. The work flow will be:
//
//          ┌───────────────┐     ┌───────────────┐      ┌───────────────┐
//          │ Authenticator │     │    Browser    │      │     Site      │
//          └───────────────┘     └───────────────┘      └───────────────┘
//                  │                     │                      │
//                  │                     │     1. Start Reg     │
//                  │                     │─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─▶│
//                  │                     │                      │
//                  │                     │     2. Challenge     │
//                  │                     │◀ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┤
//                  │                     │                      │
//                  │  3. Select Token    │                      │
//             ─ ─ ─│◀ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│                      │
//  4. Verify │     │                     │                      │
//                  │  4. Yield PubKey    │                      │
//            └ ─ ─▶│─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─▶                      │
//                  │                     │                      │
//                  │                     │  5. Send Reg Opts    │
//                  │                     │─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─▶│─ ─ ─
//                  │                     │                      │     │ 5. Verify
//                  │                     │                      │         PubKey
//                  │                     │                      │◀─ ─ ┘
//                  │                     │                      │─ ─ ─
//                  │                     │                      │     │ 6. Persist
//                  │                     │                      │       Credential
//                  │                     │                      │◀─ ─ ┘
//                  │                     │                      │
//                  │                     │                      │
//
// In this step, we are responding to the start reg(istration) request, and providing
// the challenge to the browser.

pub async fn start_register(
    auth_session: axum_login::AuthSession<Backend>,
    session: Session,
    Path(username): Path<String>,
) -> Result<impl IntoResponse, crate::error::Error> {
    info!("Start register");
    // We get the username from the URL, but you could get this via form submission or
    // some other process. In some parts of Webauthn, you could also use this as a "display name"
    // instead of a username. Generally you should consider that the user *can* and *will* change
    // their username at any time.

    // Since a user's username could change at anytime, we need to bind to a unique id.
    // We use uuid's for this purpose, and you should generate these randomly. If the
    // username does exist and is found, we can match back to our unique id. This is
    // important in authentication, where presented credentials may *only* provide
    // the unique id, and not the username!

    // let user_unique_id = {
    //     let users_guard = app_state.users.lock().await;
    //     users_guard
    //         .name_to_id
    //         .get(&username)
    //         .copied()
    //         .unwrap_or_else(Uuid::new_v4)
    // };
    let mut tx = auth_session
        .backend
        .pool
        .begin()
        .await
        .map_err(|_| Error::Unknown)?;
    let user_unique_id: Uuid = match tx.select_player_uuid(&username).await {
        Ok(_uuid) => Err(UserAlreadyRegistered(username.clone()).into()),
        Err(db::Error::Sqlx(sqlx::Error::RowNotFound)) => Ok(Uuid::new_v4()),
        Err(e) => Err(Error::from(e)),
    }?;

    // .ok()
    // .unwrap_or_else(Uuid::new_v4);

    // Remove any previous registrations that may have occured from the session.
    session.remove_value("reg_state").await.unwrap();

    // If the user has any other credentials, we exclude these here so they can't be duplicate registered.
    // It also hints to the browser that only new credentials should be "blinked" for interaction.
    // let exclude_credentials = {
    //     let users_guard = app_state.users.lock().await;
    //     users_guard
    //         .keys
    //         .get(&user_unique_id)
    //         .map(|keys| keys.iter().map(|sk| sk.cred_id().clone()).collect())
    // };

    let exclude_credentials = tx
        .select_player_passkeys(&user_unique_id)
        .await
        .map_err(|_| Error::Unknown)
        .map(|keys| keys.iter().map(|sk| sk.cred_id().clone()).collect())
        .ok();

    let res = match auth_session.backend.webauthn.start_passkey_registration(
        user_unique_id,
        &username,
        &username,
        exclude_credentials,
    ) {
        Ok((ccr, reg_state)) => {
            // Note that due to the session store in use being a server side memory store, this is
            // safe to store the reg_state into the session since it is not client controlled and
            // not open to replay attacks. If this was a cookie store, this would be UNSAFE.
            session
                .insert("reg_state", (username, user_unique_id, reg_state))
                .await
                .expect("Failed to insert");
            info!("Registration Successful!");
            Json(ccr)
        }
        Err(e) => {
            info!("challenge_register -> {:?}", e);
            return Err(Error::Unknown.into());
        }
    };
    Ok(res)
}

// 3. The browser has completed it's steps and the user has created a public key
// on their device. Now we have the registration options sent to us, and we need
// to verify these and persist them.

pub async fn finish_register(
    auth_session: axum_login::AuthSession<Backend>,
    session: Session,
    Json(reg): Json<RegisterPublicKeyCredential>,
) -> Result<impl IntoResponse, crate::error::Error> {
    let (username, user_unique_id, reg_state) = match session.get("reg_state").await? {
        Some((username, user_unique_id, reg_state)) => (username, user_unique_id, reg_state),
        None => {
            error!("Failed to get session");
            return Err(Error::CorruptSession.into());
        }
    };
    dbg!((&username, &user_unique_id, &reg_state));
    session.remove_value("reg_state").await?;

    let res = match auth_session
        .backend
        .webauthn
        .finish_passkey_registration(&reg, &reg_state)
    {
        Ok(sk) => {
            // let mut users_guard = app_state.users.lock().await;
            let mut tx = auth_session
                .backend
                .pool
                .begin()
                .await
                .map_err(|_| Error::Unknown)?;

            let is_first_player = !tx.select_exists_player().await?;

            //TODO: This is where we would store the credential in a db, or persist them in some other way.
            // users_guard
            //     .keys
            //     .entry(user_unique_id)
            //     .and_modify(|keys| keys.push(sk.clone()))
            //     .or_insert_with(|| vec![sk.clone()]);
            tx.insert_player(&username, &user_unique_id)
                .await
                .map_err(|_| Error::Unknown)?;
            tx.insert_player_passkey(&user_unique_id, &sk)
                .await
                .map_err(|e| {
                    debug!("{:?}", e);
                    Error::Unknown
                })?;

            if is_first_player {
                tx.insert_role(&user_unique_id, &Role::Adm).await?;
            }

            tx.commit().await.map_err(|e| {
                debug!("{:?}", e);
                Error::Unknown
            })?;

            // let username: String = username;
            // users_guard
            //     .name_to_id
            //     .insert(username.clone(), user_unique_id);
            // users_guard.id_to_name.insert(user_unique_id, username);

            StatusCode::OK
        }
        Err(e) => {
            error!("challenge_register -> {:?}", e);
            StatusCode::BAD_REQUEST
        }
    };

    Ok(res)
}

// 4. Now that our public key has been registered, we can authenticate a user and verify
// that they are the holder of that security token. The work flow is similar to registration.
//
//          ┌───────────────┐     ┌───────────────┐      ┌───────────────┐
//          │ Authenticator │     │    Browser    │      │     Site      │
//          └───────────────┘     └───────────────┘      └───────────────┘
//                  │                     │                      │
//                  │                     │     1. Start Auth    │
//                  │                     │─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─▶│
//                  │                     │                      │
//                  │                     │     2. Challenge     │
//                  │                     │◀ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┤
//                  │                     │                      │
//                  │  3. Select Token    │                      │
//             ─ ─ ─│◀ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│                      │
//  4. Verify │     │                     │                      │
//                  │    4. Yield Sig     │                      │
//            └ ─ ─▶│─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─▶                      │
//                  │                     │    5. Send Auth      │
//                  │                     │        Opts          │
//                  │                     │─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─▶│─ ─ ─
//                  │                     │                      │     │ 5. Verify
//                  │                     │                      │          Sig
//                  │                     │                      │◀─ ─ ┘
//                  │                     │                      │
//                  │                     │                      │
//
// The user indicates the wish to start authentication and we need to provide a challenge.

pub async fn start_authentication(
    auth_session: axum_login::AuthSession<Backend>,
    session: Session,
    Path(username): Path<String>,
) -> Result<impl IntoResponse, crate::error::Error> {
    info!("Start Authentication");
    // We get the username from the URL, but you could get this via form submission or
    // some other process.

    // Remove any previous authentication that may have occured from the session.
    let _ = session.remove_value("auth_state").await?;

    // Get the set of keys that the user possesses
    // let users_guard = app_state.users.lock().await;
    let mut tx = auth_session
        .backend
        .pool
        .begin()
        .await
        .map_err(|_| Error::Unknown)?;

    // Look up their unique id from the username
    // let user_unique_id = users_guard
    //     .name_to_id
    //     .get(&username)
    //     .copied()
    //     .ok_or(WebauthnError::UserNotFound)?;
    let user_unique_id = tx
        .select_player_uuid(&username)
        .await
        .map_err(|e| match e {
            db::Error::Sqlx(sqlx::Error::RowNotFound) => Error::UserNotFound(username),
            e => {
                error!("select_player_uuid error: {:?}", e);
                Error::Db(e)
            }
        })?;

    // let allow_credentials = users_guard
    //     .keys
    //     .get(&user_unique_id)
    //     .ok_or(WebauthnError::UserHasNoCredentials)?;
    let allow_credentials = tx
        .select_player_passkeys(&user_unique_id)
        .await
        .map_err(|_| Error::Unknown)?;

    let res = match auth_session
        .backend
        .webauthn
        .start_passkey_authentication(allow_credentials.as_slice())
    {
        Ok((rcr, auth_state)) => {
            // Drop the mutex to allow the mut borrows below to proceed
            // drop(users_guard);

            // Note that due to the session store in use being a server side memory store, this is
            // safe to store the auth_state into the session since it is not client controlled and
            // not open to replay attacks. If this was a cookie store, this would be UNSAFE.
            session
                .insert("auth_state", (user_unique_id, auth_state))
                .await
                .expect("Failed to insert");
            Json(rcr)
        }
        Err(e) => {
            info!("challenge_authenticate -> {:?}", e);
            return Err(crate::error::Error::Auth(Error::Unknown));
        }
    };
    Ok(res)
}

// 5. The browser and user have completed their part of the processing. Only in the
// case that the webauthn authenticate call returns Ok, is authentication considered
// a success. If the browser does not complete this call, or *any* error occurs,
// this is an authentication failure.

pub async fn finish_authentication(
    mut auth_session: axum_login::AuthSession<Backend>,
    session: Session,
    Json(auth): Json<PublicKeyCredential>,
) -> Result<impl IntoResponse, crate::error::Error> {
    let (_user_unique_id, auth_state): (Uuid, PasskeyAuthentication) = session
        .get("auth_state")
        .await?
        .ok_or(Error::CorruptSession)?;

    session
        .remove_value("auth_state")
        .await
        .map_err(Error::from)?;

    // TODO fix error handling
    let player = auth_session
        .authenticate((auth, auth_state))
        .await
        .map_err(|_| Error::CorruptSession)?;
    if let Some(player) = player {
        if auth_session.login(&player).await.is_err() {
            return Ok(StatusCode::UNAUTHORIZED);
        }
        Ok(StatusCode::OK)
    } else {
        Ok(StatusCode::UNAUTHORIZED)
    }
    // let res = match auth_session.backend
    //     .webauthn
    //     .finish_passkey_authentication(&auth, &auth_state)
    // {
    //     Ok(auth_result) => {
    //         // let mut users_guard = app_state.users.lock().await;
    //         let mut tx = auth_session.backend.pool.begin().await.map_err(|e| Error::Unknown)?;
    //
    //         // Update the credential counter, if possible.
    //         // TODO
    //         // users_guard
    //         //     .keys
    //         //     .get_mut(&user_unique_id)
    //         //     .map(|keys| {
    //         //         keys.iter_mut().for_each(|sk| {
    //         //             // This will update the credential if it's the matching
    //         //             // one. Otherwise it's ignored. That is why it is safe to
    //         //             // iterate this over the full list.
    //         //             sk.update_credential(&auth_result);
    //         //         })
    //         //     })
    //         //     .ok_or(WebauthnError::UserHasNoCredentials)?;
    //         tx.select_player_passkeys(&user_unique_id)
    //             .await
    //             .map(|mut keys| {
    //                 keys.iter_mut().for_each(|sk| {
    //                     // This will update the credential if it's the matching
    //                     // one. Otherwise it's ignored. That is why it is safe to
    //                     // iterate this over the full list.
    //                     sk.update_credential(&auth_result);
    //                     // TODO persist updated sk
    //                 })
    //             })
    //             .map_err(|_| Error::UserHasNoCredentials)?;
    //
    //         session.insert(AUTH_UUID, &user_unique_id).await?;
    //         StatusCode::OK
    //     }
    //     Err(e) => {
    //         info!("challenge_register -> {:?}", e);
    //         StatusCode::BAD_REQUEST
    //     }
    // };
    // info!("Authentication Successful!");
    // Ok(res)
}

// async fn logout(
//     Extension(app_state): Extension<AppState>,
//     mut auth_session: axum_login::AuthSession<Backend>,
// ) -> impl IntoResponse {
//     let _player = auth_session.logout().await.expect("logout");
//     HtmlTemplate(LoginTemplate {
//     })
// }

pub async fn logout(mut auth_session: AuthSession) -> impl IntoResponse {
    match auth_session.logout().await {
        Ok(_) => {
            let mut response = StatusCode::OK.into_response();
            response
                .headers_mut()
                .insert("HX-Refresh", HeaderValue::from_static("true"));
            response
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

impl AuthUser for Player {
    type Id = Uuid;

    fn id(&self) -> Self::Id {
        self.uuid
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.passkeys
            .first()
            .expect("at least one passkey")
            .cred_id()
            .as_slice()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Backend {
    pub pool: Pool<Sqlite>,
    pub webauthn: Arc<Webauthn>,
}

impl Backend {
    pub(crate) fn new(pool: Pool<Sqlite>, domain_name: String, web_url: String) -> Self {
        // Effective domain name.
        let rp_id = domain_name.as_str();
        // Url containing the effective domain name
        // MUST include the port number!
        let rp_origin = Url::parse(web_url.as_str()).expect("Invalid URL");
        let builder = WebauthnBuilder::new(rp_id, &rp_origin).expect("Invalid configuration");

        // Now, with the builder you can define other options.
        // Set a "nice" relying party name. Has no security properties and
        // may be changed in the future.
        let builder = builder.rp_name("Nonce Guess");

        // Consume the builder and create our webauthn instance.
        let webauthn = Arc::new(builder.build().expect("Invalid configuration"));

        Self { pool, webauthn }
    }
}

// We use a type alias for convenience.
//
// Note that we've supplied our concrete backend here.
pub type AuthSession = axum_login::AuthSession<Backend>;

#[async_trait]
impl AuthnBackend for Backend {
    type User = Player;
    type Credentials = (PublicKeyCredential, PasskeyAuthentication);
    type Error = Error;

    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        // let mut tx = self
        //     .pool
        //     .begin()
        //     .await
        //     .map_err(db::Error::Sqlx)
        //     .map_err(Error::Db)?;
        // tx.select_player_by_credential(&creds)
        //     .await
        //     .map_err(Error::Db)

        let (auth, auth_state) = creds;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(db::Error::from)
            .map_err(Error::from)?;

        let uuid = auth
            .get_user_unique_id()
            .map(|uuid_bytes| Uuid::from_slice(uuid_bytes).expect("uuid"));

        if let Some(user_unique_id) = uuid {
            match self
                .webauthn
                .finish_passkey_authentication(&auth, &auth_state)
            {
                Ok(auth_result) => {
                    // let mut users_guard = app_state.users.lock().await;

                    // Update the credential counter, if possible.
                    // TODO
                    // users_guard
                    //     .keys
                    //     .get_mut(&user_unique_id)
                    //     .map(|keys| {
                    //         keys.iter_mut().for_each(|sk| {
                    //             // This will update the credential if it's the matching
                    //             // one. Otherwise it's ignored. That is why it is safe to
                    //             // iterate this over the full list.
                    //             sk.update_credential(&auth_result);
                    //         })
                    //     })
                    //     .ok_or(WebauthnError::UserHasNoCredentials)?;
                    tx.select_player_passkeys(&user_unique_id)
                        .await
                        .map(|mut keys| {
                            keys.iter_mut().for_each(|sk| {
                                // This will update the credential if it's the matching
                                // one. Otherwise it's ignored. That is why it is safe to
                                // iterate this over the full list.
                                sk.update_credential(&auth_result);
                                // TODO persist updated sk
                            })
                        })
                        .map_err(|_| Error::UserHasNoCredentials)?;

                    // session.insert(AUTH_UUID, &user_unique_id).await?;
                    // StatusCode::OK
                    let player = tx
                        .select_player_by_uuid(&user_unique_id)
                        .await
                        .expect("player"); // TODO fix error handling
                    info!("Authentication Successful!");
                    Ok(player)
                }
                Err(e) => {
                    info!("challenge_register -> {:?}", e);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(db::Error::Sqlx)
            .map_err(Error::Db)?;

        tx.select_player_by_uuid(user_id).await.map_err(Error::Db)
    }
}

// Permissions that can be granted to a player.
#[derive(Debug, Clone, Eq, PartialEq, Hash, strum_macros::Display, strum_macros::EnumString)]
pub(crate) enum Permission {
    /// Assign a player to the admin role.
    AssignAdm,
    /// Assign a player to the moderator role.
    AssignMod,
    /// Change the target block height.
    ChangeTargetBlock,
}

// Roles (with permissions) that can be granted to a player.
#[derive(Debug, Clone, Eq, PartialEq, Hash, strum_macros::Display, strum_macros::EnumString)]
pub(crate) enum Role {
    /// Administrator.
    Adm,
    /// Moderator.
    Mod,
}

impl Role {
    fn permissions(&self) -> Vec<Permission> {
        match &self {
            Role::Adm => vec![
                Permission::AssignAdm,
                Permission::AssignMod,
                Permission::ChangeTargetBlock,
            ],
            Role::Mod => vec![Permission::ChangeTargetBlock],
        }
    }
}

#[async_trait]
impl AuthzBackend for Backend {
    type Permission = Permission;

    async fn get_user_permissions(
        &self,
        user: &Self::User,
    ) -> Result<HashSet<Self::Permission>, Self::Error> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(db::Error::Sqlx)
            .map_err(Error::Db)?;

        tx.select_permissions(&user.uuid).await.map_err(Error::from)
    }

    async fn get_group_permissions(
        &self,
        user: &Self::User,
    ) -> Result<HashSet<Self::Permission>, Self::Error> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(db::Error::Sqlx)
            .map_err(Error::Db)?;

        let roles = tx.select_roles(&user.uuid).await.map_err(Error::from)?;
        let permissions = roles.into_iter().flat_map(|r| r.permissions()).collect();
        Ok(permissions)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("unknown webauthn error")]
    Unknown,
    #[error("corrupt Session")]
    CorruptSession,
    #[error("user not found")]
    UserNotFound(String),
    #[error("user already registered")]
    UserAlreadyRegistered(String),
    #[error("user has no credentials")]
    UserHasNoCredentials,
    #[error("deserializing session failed: {0}")]
    InvalidSessionState(#[from] tower_sessions::session::Error),
    #[error("db: {0}")]
    Db(#[from] db::Error),
}
