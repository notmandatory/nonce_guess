use askama::Template;
use axum::extract::{Path, State};
use axum::handler::HandlerWithoutStateExt;
use axum::http::{header, Uri};
use axum::response::{Html, Response};
use axum::routing::{get, put};
use axum::{
    error_handling::HandleErrorLayer, extract::Extension, http::StatusCode, response::IntoResponse,
    routing::post, BoxError, Form, Json, Router,
};
use axum_embed::ServeEmbed;
use axum_login::tower_sessions::service::PlaintextCookie;
use axum_login::{AuthManagerLayerBuilder, AuthSession};
use clap::Parser;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{migrate, Pool, Sqlite};
use std::net::SocketAddr;
use std::ops::{Add, Deref};
use std::str::FromStr;
use std::sync::Arc;
use tower::{Layer, ServiceBuilder};
use tower_sessions::SessionStore;
use tower_sessions::{
    cookie::{time::Duration, SameSite},
    Expiry, MemoryStore, Session, SessionManagerLayer,
};
use tower_sessions_sqlx_store::SqliteStore;
use tracing::{debug, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use webauthn_rs::prelude::{PasskeyAuthentication, PasskeyRegistration, PublicKeyCredential, Uuid};

// The handlers that process the data can be found in the auth.rs file
// This file contains the wasm client loading code and the axum routing
// use crate::auth::{
//     finish_authentication, finish_register, start_authentication, start_register, Backend, Error,
//     AUTH_UUID,
// };
use crate::db::Db;
use crate::model::{Block, Guess, Player, Target};
// use crate::startup::AppState;
use crate::web::App;

mod db;
mod error;
mod model;
// mod startup;
mod web;

// const COUNTER_KEY: &str = "counter";

// #[derive(Default, Deserialize, Serialize)]
// struct Counter(usize);

// #[derive(RustEmbed, Clone)]
// #[folder = "./assets"]
// struct Assets;

// static INDEX_HTML: &str = "index.html";
// static GUESSED_BLOCK: &str = "guessed_block";

// pub struct AppState {
//     pub pool: Pool<Sqlite>,
// }

/// Nonce Guess Server CLI arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    /// Address this server should listen on, defaults to "localhost:8081"
    #[arg(short, long, value_name = "HOST:PORT")]
    listen_address: Option<String>,
    #[arg(short, long, value_name = "DB_URL", env = "NG_DB_URL")]
    /// SQLite DB URL for this server, ie. "sqlite://nonce_guess.sqlite", defaults to in-memory DB
    database_url: Option<String>,
}

//struct SqliteStore(Pool<Sqlite>);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(EnvFilter::new(std::env::var("RUST_LOG").unwrap_or_else(
            |_| "axum_login=debug,tower_sessions=debug,sqlx=warn,tower_http=debug".into(),
        )))
        .with(tracing_subscriber::fmt::layer())
        .try_init()?;

    // TODO is "?mode=rwc" required?
    // get database URL from env
    let database_url = std::env::var("NONCE_GUESS_DB_URL").ok();
    debug!("database_url: {:?}", &database_url);

    App::new(database_url).await?.serve().await

    // // setup connection pool
    // let pool = if let Some(url) = database_url {
    //     SqlitePoolOptions::new()
    //         .connect(&url)
    //         .await
    //         .expect("connect to file backed database")
    // } else {
    //     // this should only be used for testing
    //     SqlitePoolOptions::new()
    //         .max_connections(1)
    //         .min_connections(1)
    //         .idle_timeout(None)
    //         .max_lifetime(None)
    //         .connect(":memory:")
    //         .await
    //         .expect("connect to memory backed database")
    // };

    // migrate!("./migrations")
    //     .run(&pool)
    //     .await
    //     .expect("migrated database");

    // let app_state = AppState::new(pool.clone());

    // Session store.
    //let session_store = SqliteStore::<SessionSqlitePool>::new(Some(pool.clone().into()), session_store_config).await.unwrap();
    //let session_store = SqliteStore::new(pool.clone());
    // let session_store = SqliteStore::new(pool.clone());
    // session_store.migrate().await.expect("migrated session_store");

    // Session manager layer.
    // let session_manager_layer = SessionManagerLayer::new(session_store)
    //     .with_name("webauthnrs")
    //     .with_same_site(SameSite::Strict)
    //     // TODO: change this to true when running on an HTTPS/production server instead of locally
    //     .with_secure(false)
    //     .with_expiry(Expiry::OnInactivity(Duration::seconds(360)));

    // // Auth service.
    // //
    // // This combines the session layer with our backend to establish the auth
    // // service which will provide the auth session as a request extension.
    // let backend = Backend::new(pool.clone());
    // let auth_layer = AuthManagerLayerBuilder::new(backend, session_manager_layer).build();

    // let session_service = ServiceBuilder::new()
    //     .layer(HandleErrorLayer::new(|_: BoxError| async {
    //         StatusCode::BAD_REQUEST
    //     }))
    //     .layer(
    //         SessionManagerLayer::new(session_store)
    //             .with_name("webauthnrs")
    //             .with_same_site(SameSite::Strict)
    //             .with_secure(false) // TODO: change this to true when running on an HTTPS/production server instead of locally
    //             .with_expiry(Expiry::OnInactivity(Duration::seconds(360))),
    //     );

    // let api_routes = Router::new()
    //     .route("/target", get(get_current_target).post(post_target_block))
    //     .route("/target/nonce", get(get_target_nonce))
    //     .route("/guesses", get(get_guesses).post(post_guess))
    //     .route("/guesses/:block", get(get_block_guesses))
    //     .layer(Extension(shared_state));

    // build our application with a route
    // let app = Router::new()
    //     // .route("/guesses", post(post_guess))
    //     // .nest("/api", api_routes)
    //     // .nest("/assets", static_handler.into_service())
    //     .layer(CookieManagerLayer::new());

    // let app = Router::new()
    //     .route("/", get(home).post(post_guess))
    //     .route("/logout", put(logout))
    //     .route("/register_start/:username", post(start_register))
    //     .route("/register_finish", post(finish_register))
    //     .route("/login_start/:username", post(start_authentication))
    //     .route("/login_finish", post(finish_authentication))
    //     .nest_service("/assets", serve_assets)
    //     .layer(Extension(app_state))
    //     .layer(auth_layer)
    //     // .layer(CookieManagerLayer::new())
    //     .fallback(handler_404);

    // let app = Router::new()
    //     .merge(app)
    //     .nest_service("/assets", tower_http::services::ServeDir::new("/assets"));

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    // let listen_address = cli_args
    //     .listen_address
    //     .unwrap_or_else(|| "127.0.0.1:8081".to_string());
    //
    // let addr = SocketAddr::from_str(listen_address.as_str()).unwrap();
    // info!("listening on {}", addr);
    //
    // let listener = tokio::net::TcpListener::bind(listen_address).await.unwrap();
    // axum::serve(listener, app).await.unwrap();
}

struct HtmlTemplate<T>(T);

// #[derive(Template, Default, Debug)]
// #[template(path = "pages/home.html")]
// struct HomeTemplate {
//     pub uuid: Option<Uuid>,
//     pub name: Option<String>,
//     pub target: Option<Target>,
//     pub guesses: Vec<Guess>,
//     pub my_guess: Option<Guess>,
// }

// async fn handler(cookies: Cookies) -> String {
//     cookies.add(Cookie::new("hello_world", "hello_world"));
//
//     let guessed_block = cookies.get("guessed_block")
//         .map(|c| u32::from_str(c.value()).ok())
//         .unwrap_or(None);
//     info!("{:?}", &guessed_block);
//
//     format!("Check your cookies. {:?}", guessed_block)
// }

// async fn home(
//     mut auth_session: AuthSession<Backend>,
//     Extension(app_state): Extension<AppState>,
// ) -> impl IntoResponse {
//     // get users current guessed block from cookie
//     // let guessed_block: Option<u32> = cookies
//     //     .get(GUESSED_BLOCK)
//     //     .map(|c| u32::from_str(c.value()).ok())
//     //     .unwrap_or(None);
//     // debug!("{:?}", &guessed_block);
//
//     // let uuid: Option<Uuid> = session.get(AUTH_UUID).expect("can't get AUTH_UUID");
//     let player: Option<Player> = auth_session.user;
//     info!("Current player: {:?}", player);
//
//     let home = if let Some(some_player) = player {
//         let uuid = some_player.uuid;
//         let mut tx = app_state.pool.begin().await.expect("tx");
//         let name = tx.select_player_name(&uuid).await.ok();
//         let target = tx.select_current_target().await.ok();
//         let guesses = if let Some(some_target) = &target {
//             tx.select_block_guesses(some_target.block).await.ok()
//         } else {
//             tx.select_guesses().await.ok()
//         }
//         .unwrap_or_default();
//         let my_guess: Option<Guess> = guesses
//             .iter()
//             .find(|g| g.uuid == uuid)
//             .map(|guess| guess.clone());
//
//         HomeTemplate {
//             uuid: Some(uuid),
//             name,
//             target,
//             guesses,
//             my_guess,
//         }
//     } else {
//         Default::default()
//     };
//
//     dbg!(&home);
//     HtmlTemplate(home)
//
//     // info!("session: {:?}", &session);
//     // let reg_state: Option<(String, Uuid, PasskeyRegistration)> = session.get("reg_state").unwrap();
//     // info!("session.get('reg_state'): {:?}", reg_state);
//     // let auth_state: Option<(Uuid, PasskeyRegistration)> = session.get("auth_state").unwrap();
//     // info!("session.get('auth_state'): {:?}", auth_state);
//     // info!("users.name_to_id: {:?}", &app_state.users.try_lock().unwrap().name_to_id);
//     // info!("users.id_to_name: {:?}", &app_state.users.try_lock().unwrap().id_to_name);
//     //info!("users.keys: {:?}", &app_state.users.try_lock().unwrap().keys);
//
//     // // begin a db transaction
//     // match state.pool.begin().await {
//     //     Ok(mut tx) => {
//     //         // get the current target (block and nonce)
//     //         let target = tx.select_current_target().await.ok();
//     //         // TODO get current guesses
//     //         // return home template
//     //         HtmlTemplate(HomeTemplate {
//     //             target,
//     //             guessed_block,
//     //             name_error: None,
//     //             nonce_error: None,
//     //             guesses: Vec::new(),
//     //         })
//     //     }
//     //     // return error if can't begin db transaction
//     //     Err(e) => (
//     //         StatusCode::INTERNAL_SERVER_ERROR,
//     //         format!("Failed to render template. Error: {}", e.to_string()),
//     //     ),
//     // }
// }
//
//
// async fn logout(
//     Extension(app_state): Extension<AppState>,
//     mut auth_session: AuthSession<Backend>,
// ) -> impl IntoResponse {
//     let _player = auth_session.logout().await.expect("logout");
//     HtmlTemplate(HomeTemplate {
//         uuid: None,
//         name: None,
//         target: None,
//         guesses: Vec::new(),
//         my_guess: None,
//     })
// }
//
// #[derive(Serialize, Deserialize, Default, Debug, Clone, Eq, PartialEq)]
// struct GuessRequest {
//     pub name: String,
//     pub guess: String,
// }
//
// #[derive(Deserialize)]
// struct GuessForm {
//     guess: String,
// }
//
// async fn post_guess(
//     Extension(app_state): Extension<AppState>,
//     mut auth_session: AuthSession<Backend>,
//     Form(guess_form): Form<GuessForm>,
// ) -> Result<impl IntoResponse, Error> {
//     let guess = guess_form.guess;
//     dbg!(&guess);
//
//     //let uuid: Option<Uuid> = session.get(AUTH_UUID).expect("can't get AUTH_UUID");
//
//     let player: Option<Player> = auth_session.user;
//     info!("Current player: {:?}", player);
//
//     let home = if let Some(some_player) = player {
//         let uuid = some_player.uuid;
//         let mut tx = app_state.pool.begin().await.expect("tx");
//         let name = tx.select_player_name(&uuid).await.ok();
//         let target = tx.select_current_target().await.ok();
//
//         // add guess
//         let block = target.as_ref().map(|t| t.block);
//         let nonce = u32::from_str_radix(guess.as_str(), 16).map_err(|e| Error::InvalidInput)?;
//         tx.insert_guess(&uuid, block, nonce)
//             .await
//             .map_err(|e| Error::Db(e))?;
//         tx.commit().await.expect("commit");
//
//         let mut tx = app_state.pool.begin().await.expect("tx");
//         let guesses = if let Some(some_target) = &target {
//             tx.select_block_guesses(some_target.block).await.ok()
//         } else {
//             tx.select_guesses().await.ok()
//         }
//         .unwrap_or_default();
//
//         let my_guess = Some(Guess {
//             uuid,
//             block,
//             name: name.clone().unwrap(),
//             nonce,
//         });
//
//         HomeTemplate {
//             uuid: Some(uuid),
//             name,
//             target,
//             guesses,
//             my_guess,
//         }
//     } else {
//         Default::default()
//     };
//
//     dbg!(&home);
//     Ok(HtmlTemplate(home))
// }

// async fn post_guess(
//     cookies: Cookies,
//     State(state): State<Arc<AppState>>,
//     Form(guess_request): Form<GuessRequest>,
// ) -> impl IntoResponse {
//     // get users current guessed block from cookie
//     let guessed_block: Option<u32> = cookies
//         .get(GUESSED_BLOCK)
//         .map(|c| u32::from_str(c.value()).ok())
//         .unwrap_or(None);
//     debug!("{:?}", &guessed_block);
//
//     // create db transaction
//     match state.pool.begin().await {
//         Ok(mut tx) => {
//             // TODO make sure name is unique
//             // convert guess from hex string to decimal nonce
//             let nonce = u32::from_str_radix(guess_request.guess.as_str(), 16);
//
//             // get the current target
//             let target = tx.select_current_target().await;
//
//             match (guessed_block, target) {
//                 // guessed block less than current target
//                 (Some(g), Ok(t)) if g < t.block => {
//                     // update cookie
//                     cookies.add(Cookie::new(GUESSED_BLOCK, t));
//                     // add guess
//                     // new template
//                 },
//                 // no guessed block or not less than current target
//                 (_, Ok(t)) => {
//                     // new template
//                 },
//                 // guessed block, no current target found
//                 (Some(g), Err(Error::Db(e))) if e == RowNotFound => {
//                     // update cookie
//                     cookies.add(Cookie::new(GUESSED_BLOCK, 0));
//                     // add guess
//                     tx.insert_guess(Guess {
//                         block: None,
//                         name: "".to_string(),
//                         nonce: 0,
//                     }).await
//                     // new template
//                     HtmlTemplate(HomeTemplate {
//                         target: None,
//                         guessed_block: Some(g),
//                         name_error: None,
//                         nonce_error: None,
//                         guesses: Vec::new(),
//                     })
//                 },
//                 // some other error loading current target
//                 (_, Err(e)) => (
//                     StatusCode::INTERNAL_SERVER_ERROR,
//                     format!("Failed to render template. Error: {}", e.to_string()),
//                 ),
//             }
//
//             // match (nonce, target) {
//             //     (Ok(nonce), Ok(target)) => {
//             //         let target = tx.select_current_target().await.ok();
//             //
//             //         info!("{:?}", &guessed_block);
//             //
//             //         let guess = Guess {
//             //             name: guess_request.name,
//             //             nonce: nonce,
//             //         };
//             //         tx.insert_guess(guess).await?;
//             //         tx.commit().await?;
//             //
//             //         HtmlTemplate(HomeTemplate {
//             //             target: target,
//             //             guessed_block,
//             //             name_error: None,
//             //             nonce_error: None,
//             //             guesses: Vec::new(),
//             //         })
//             //     }
//             //     Err(error) => {}
//             // }
//         }
//         Err(e) => (
//             StatusCode::INTERNAL_SERVER_ERROR,
//             format!("Failed to render template. Error: {}", e.to_string()),
//         ),
//     }
// }

// impl<T> IntoResponse for HtmlTemplate<T>
// where
//     T: Template,
// {
//     fn into_response(self) -> Response {
//         match self.0.render() {
//             Ok(html) => Html(html).into_response(),
//             Err(err) => (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 format!("Failed to render template. Error: {}", err),
//             )
//                 .into_response(),
//         }
//     }
// }

// async fn static_handler(uri: Uri) -> Response {
//     let path = uri.path().trim_start_matches('/');
//
//     // if path.is_empty() || path == INDEX_HTML {
//     //     return index_html().await;
//     // }
//
//     match Assets::get(path) {
//         Some(content) => {
//             let body = tower_http::body::Full::from(content.data);
//             let mime = mime_guess::from_path(path).first_or_octet_stream();
//
//             Response::builder()
//                 .header(header::CONTENT_TYPE, mime.as_ref())
//                 .body(body)
//                 .unwrap()
//         }
//         None => {
//             if path.contains('.') {
//                 return not_found().await;
//             }
//
//             index_html().await
//         }
//     }
// }

// async fn index_html() -> Response {
//     match Assets::get(INDEX_HTML) {
//         Some(content) => {
//             let body = boxed(Full::from(content.data));
//
//             Response::builder()
//                 .header(header::CONTENT_TYPE, "text/html")
//                 .body(body)
//                 .unwrap()
//         }
//         None => not_found().await,
//     }
// }
//
// async fn not_found() -> Response {
//     Response::builder()
//         .status(StatusCode::NOT_FOUND)
//         .body(boxed(Full::from("404")))
//         .unwrap()
// }

// async fn get_current_target(State(state): State<Arc<AppState>>) -> Result<Json<Target>, Error> {
//     let mut tx = state.pool.begin().await?;
//
//     if let Ok(target) = tx.select_current_target().await {
//         tx.commit().await?;
//         Ok(Json(target))
//     } else {
//         tx.commit().await?;
//         Err(Error::Generic("No target block is set.".to_string()))
//     }
// }

// async fn post_target_block(State(state): State<Arc<AppState>>, block: String) -> Result<(), Error> {
//     let new_target = u32::from_str(block.as_str())?;
//     let mut tx = state.pool.begin().await?;
//     let current_target = tx.select_current_target().await.unwrap_or_default();
//     if current_target.block < new_target {
//         tx.insert_target(new_target).await?;
//         tx.commit().await?;
//         Ok(())
//     } else {
//         Err(Error::Generic(
//             "New target block must be greater than current target.".to_string(),
//         ))
//     }
// }

// async fn get_target_nonce(State(state): State<Arc<AppState>>) -> Result<String, Error> {
//     //let nonce = u32::from_str(nonce.as_str())?;
//     let client = reqwest::Client::new();
//     let mut tx = state.pool.begin().await?;
//     let current_target = tx.select_current_target().await?;
//     let block_height_response = client
//         .get(format!(
//             "https://mempool.space/api/block-height/{}",
//             current_target.block
//         ))
//         .send()
//         .await?;
//     if block_height_response.status().is_success() {
//         let block_hash = block_height_response.text().await?;
//         let block_response = client
//             .get(format!("https://mempool.space/api/block/{}", block_hash))
//             .send()
//             .await?;
//         if block_response.status().is_success() {
//             let block: Block = block_response.json().await?;
//             let nonce = block.nonce;
//             tx.set_current_nonce(nonce).await?;
//             tx.set_guesses_block(block.height).await?;
//             tx.commit().await?;
//             return Ok(nonce.to_string());
//         }
//     }
//     Ok(String::default())
// }

// async fn get_guesses(State(state): State<Arc<AppState>>) -> Result<Json<Vec<Guess>>, Error> {
//     let mut tx = state.pool.begin().await?;
//     let guesses = tx.select_guesses().await.unwrap_or_default();
//     tx.commit().await?;
//     Ok(Json(guesses))
// }

// async fn get_block_guesses(
//     State(state): State<Arc<AppState>>,
//     Path(block): Path<u32>,
// ) -> Result<Json<Vec<Guess>>, Error> {
//     let mut tx = state.pool.begin().await?;
//     let guesses = tx.select_block_guesses(block).await?;
//     tx.commit().await?;
//     Ok(Json(guesses))
// }

// async fn post_guess(
//     Extension(state): Extension<Arc<State>>,
//     Json(guess): Json<Guess>,
// ) -> Result<(), Error> {
//     let mut tx = state.pool.begin().await?;
//     tx.insert_guess(guess).await?;
//     tx.commit().await?;
//     Ok(())
// }

// impl IntoResponse for auth::Error {
//     fn into_response(self) -> Response {
//         let (status_code, body) = match self {
//             auth::Error::CorruptSession => (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 "corrupt session".to_string(),
//             ),
//             auth::Error::UserNotFound(username) => (StatusCode::NOT_FOUND, username),
//             auth::Error::UserAlreadyRegistered(username) => (StatusCode::BAD_REQUEST, username),
//             auth::Error::Unknown => (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 "unknown error".to_string(),
//             ),
//             auth::Error::UserHasNoCredentials => (
//                 StatusCode::UNAUTHORIZED,
//                 "user has no credentials".to_string(),
//             ),
//             auth::Error::InvalidSessionState(_) => (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 "deserializing session failed".to_string(),
//             ),
//             auth::Error::Db(_) => (StatusCode::INTERNAL_SERVER_ERROR, "db error".to_string()),
//             auth::Error::InvalidInput => (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 "invalid input".to_string(),
//             ),
//         };
//
//         // its often easiest to implement `IntoResponse` by calling other implementations
//         (status_code, body).into_response()
//     }
// }
//
// async fn handler_404() -> impl IntoResponse {
//     (StatusCode::NOT_FOUND, "nothing to see here")
// }
