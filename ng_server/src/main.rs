use askama::Template;
use axum::body::{boxed, Full};
use axum::extract::{Path, State};
use axum::handler::HandlerWithoutStateExt;
use axum::http::{header, Uri};
use axum::response::{Html, Response};
use axum::routing::{get, put};
use axum::{
    error_handling::HandleErrorLayer, extract::Extension, http::StatusCode, response::IntoResponse,
    routing::post, BoxError, Json, Router,
};
use clap::Parser;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{migrate, Pool, Sqlite};
use std::net::SocketAddr;
use std::ops::{Add, Deref};
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_sessions::{
    cookie::{time::Duration, SameSite},
    Expiry, MemoryStore, Session, SessionManagerLayer, SqliteStore,
};
use tracing::{debug, info};
use webauthn_rs::prelude::{PasskeyRegistration, PublicKeyCredential, Uuid};

// The handlers that process the data can be found in the auth.rs file
// This file contains the wasm client loading code and the axum routing
use crate::auth::{
    finish_authentication, finish_register, start_authentication, start_register, AUTH_UUID,
};
use crate::db::Db;
use crate::error::Error;
use crate::model::{Block, Guess, Target};
use crate::startup::AppState;

mod auth;
mod db;
mod error;
mod model;
mod startup;

const COUNTER_KEY: &str = "counter";

#[derive(Default, Deserialize, Serialize)]
struct Counter(usize);

#[derive(RustEmbed)]
#[folder = "./assets"]
struct Assets;

static INDEX_HTML: &str = "index.html";
static GUESSED_BLOCK: &str = "guessed_block";

// pub struct AppState {
//     pub pool: Pool<Sqlite>,
// }

/// Nonce Guess Server CLI arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    /// Address this server should listen on, defaults to "127.0.0.1:8081"
    #[arg(short, long, value_name = "HOST:PORT")]
    listen_address: Option<String>,
    #[arg(short, long, value_name = "NG_DB_URL")]
    /// SQLite DB URL for this server, ie. "sqlite://nonce_guess.db", defaults to in-memory DB
    database_url: Option<String>,
}

//struct SqliteStore(Pool<Sqlite>);

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    // Parse CLI arguments
    let cli_args: CliArgs = CliArgs::parse();

    // get database URL from env
    let database_url = cli_args
        .database_url
        .unwrap_or_else(|| ":memory:".to_string())
        .add("?mode=rwc");

    // setup connection pool
    let pool = SqlitePoolOptions::new()
        .connect(&database_url)
        .await
        .expect("can connect to database");

    migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrated database");

    let app_state = AppState::new(pool.clone());

    //let session_store = MemoryStore::default();
    let session_store = SqliteStore::new(pool);
    session_store
        .migrate()
        .await
        .expect("couldn't migrate session store");

    let session_service = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|_: BoxError| async {
            StatusCode::BAD_REQUEST
        }))
        .layer(
            SessionManagerLayer::new(session_store)
                .with_name("webauthnrs")
                .with_same_site(SameSite::Strict)
                .with_secure(false) // TODO: change this to true when running on an HTTPS/production server instead of locally
                .with_expiry(Expiry::OnInactivity(Duration::seconds(360))),
        );

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

    let app = Router::new()
        .route("/", get(home)) //.post(post_guess))
        .route("/logout", put(logout))
        .route("/register_start/:username", post(start_register))
        .route("/register_finish", post(finish_register))
        .route("/login_start/:username", post(start_authentication))
        .route("/login_finish", post(finish_authentication))
        // //.with_state(app_state)
        .layer(Extension(app_state))
        .layer(session_service)
        // .layer(CookieManagerLayer::new())
        .fallback(handler_404);

    let app = Router::new()
        .merge(app)
        .nest_service("/assets", static_handler.into_service());

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let listen_address = cli_args
        .listen_address
        .unwrap_or_else(|| "127.0.0.1:8081".to_string());

    let addr = SocketAddr::from_str(listen_address.as_str()).unwrap();
    info!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

struct HtmlTemplate<T>(T);

#[derive(Template)]
#[template(path = "pages/home.html")]
struct HomeTemplate {
    pub uuid: Option<Uuid>,
    pub name: Option<String>,
    pub target: Option<Target>,
    pub guessed_block: Option<u32>,
    pub guesses: Vec<Guess>,
    pub name_error: Option<String>,
    pub nonce_error: Option<String>,
}

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

async fn home(Extension(app_state): Extension<AppState>, session: Session) -> impl IntoResponse {
    // get users current guessed block from cookie
    // let guessed_block: Option<u32> = cookies
    //     .get(GUESSED_BLOCK)
    //     .map(|c| u32::from_str(c.value()).ok())
    //     .unwrap_or(None);
    // debug!("{:?}", &guessed_block);
    let uuid: Option<Uuid> = session.get(AUTH_UUID).expect("can't get AUTH_UUID");
    info!("Current uuid: {:?}", uuid);
    // let name: Option<String> = uuid.map(|u| app_state.users.try_lock().unwrap().id_to_name.get(&u).cloned().unwrap());
    // info!("Current user: {:?}", name);
    let name = None;

    info!("session: {:?}", &session);
    let reg_state: Option<(String, Uuid, PasskeyRegistration)> = session.get("reg_state").unwrap();
    info!("session.get('reg_state'): {:?}", reg_state);
    let auth_state: Option<(Uuid, PasskeyRegistration)> = session.get("auth_state").unwrap();
    info!("session.get('auth_state'): {:?}", auth_state);
    // info!("users.name_to_id: {:?}", &app_state.users.try_lock().unwrap().name_to_id);
    // info!("users.id_to_name: {:?}", &app_state.users.try_lock().unwrap().id_to_name);
    //info!("users.keys: {:?}", &app_state.users.try_lock().unwrap().keys);

    HtmlTemplate(HomeTemplate {
        uuid,
        name,
        target: None,
        guessed_block: None,
        name_error: None,
        nonce_error: None,
        guesses: Vec::new(),
    })

    // // begin a db transaction
    // match state.pool.begin().await {
    //     Ok(mut tx) => {
    //         // get the current target (block and nonce)
    //         let target = tx.select_current_target().await.ok();
    //         // TODO get current guesses
    //         // return home template
    //         HtmlTemplate(HomeTemplate {
    //             target,
    //             guessed_block,
    //             name_error: None,
    //             nonce_error: None,
    //             guesses: Vec::new(),
    //         })
    //     }
    //     // return error if can't begin db transaction
    //     Err(e) => (
    //         StatusCode::INTERNAL_SERVER_ERROR,
    //         format!("Failed to render template. Error: {}", e.to_string()),
    //     ),
    // }
}

async fn logout(Extension(app_state): Extension<AppState>, session: Session) -> impl IntoResponse {
    session.flush();
    HtmlTemplate(HomeTemplate {
        uuid: None,
        name: None,
        target: None,
        guessed_block: None,
        name_error: None,
        nonce_error: None,
        guesses: Vec::new(),
    })
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Eq, PartialEq)]
struct GuessRequest {
    pub name: String,
    pub guess: String,
}

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

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {}", err),
            )
                .into_response(),
        }
    }
}

async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    if path.is_empty() || path == INDEX_HTML {
        return index_html().await;
    }

    match Assets::get(path) {
        Some(content) => {
            let body = boxed(Full::from(content.data));
            let mime = mime_guess::from_path(path).first_or_octet_stream();

            Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(body)
                .unwrap()
        }
        None => {
            if path.contains('.') {
                return not_found().await;
            }

            index_html().await
        }
    }
}

async fn index_html() -> Response {
    match Assets::get(INDEX_HTML) {
        Some(content) => {
            let body = boxed(Full::from(content.data));

            Response::builder()
                .header(header::CONTENT_TYPE, "text/html")
                .body(body)
                .unwrap()
        }
        None => not_found().await,
    }
}

async fn not_found() -> Response {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(boxed(Full::from("404")))
        .unwrap()
}

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

async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "nothing to see here")
}
