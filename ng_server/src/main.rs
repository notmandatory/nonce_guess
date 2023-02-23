use axum::body::{boxed, Full};
use axum::extract::{Extension, Path};
use axum::handler::Handler;
use axum::http::{header, Uri};
use axum::response::{IntoResponse, Response, Result};
use axum::{http::StatusCode, routing::get, Json, Router};

use axum::routing::{on, MethodFilter};
use std::net::SocketAddr;
use std::ops::Add;
use std::str::FromStr;
use std::sync::Arc;

use ng_model::*;

use sqlx::{migrate, Pool, Sqlite};

use sqlx::sqlite::SqlitePoolOptions;
use tracing::info;

use crate::db::Db;
use crate::error::Error;
use crate::Error::Generic;
use rust_embed::RustEmbed;

use clap::Parser;

mod db;
mod error;

#[derive(RustEmbed)]
#[folder = "../ng_web/dist"]
struct Assets;

static INDEX_HTML: &str = "index.html";

pub struct State {
    pub pool: Pool<Sqlite>,
}

/// Nonce Guess Server CLI arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    // /// Public URL of this server, if set will be displayed as a QR code
    // #[arg(short, long, value_name = "NG_WEB_URL")]
    // web_url: Option<String>,
    #[arg(short, long, value_name = "NG_DB_URL")]
    /// SQLite DB URL for this server, ie. "sqlite://nonce_guess.db", defaults to in-memory DB
    db_url: Option<String>,
}

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    // Parse CLI arguments
    let cli_args: CliArgs = CliArgs::parse();
    dbg!(&cli_args);

    // get database URL from env
    let database_url = cli_args
        .db_url
        .unwrap_or_else(|| ":memory:".to_string())
        .add("?mode=rwc");
    dbg!(&database_url);

    // setup connection pool
    let pool = SqlitePoolOptions::new()
        .connect(&database_url)
        .await
        .expect("can connect to database");

    migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrated database");

    let shared_state = Arc::new(State { pool });

    let api_routes = Router::new()
        .route("/target", get(get_current_target).post(post_target_block))
        .route("/target/nonce", get(get_target_nonce))
        .route("/guesses/:block", get(get_guesses))
        .route("/guesses", on(MethodFilter::POST, post_guess))
        .layer(Extension(shared_state));

    // build our application with a route
    let app = Router::new()
        .nest("/api", api_routes)
        .fallback(static_handler.into_service());

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([127, 0, 0, 1], 8081));
    info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
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

async fn get_current_target(
    Extension(state): Extension<Arc<State>>,
) -> Result<Json<Target>, Error> {
    let mut tx = state.pool.begin().await?;
    if let Ok(target) = tx.select_current_target().await {
        tx.commit().await?;
        Ok(Json(target))
    } else {
        let client = reqwest::Client::new();
        let block_tip_height_response = client
            .get(format!("https://mempool.space/api/blocks/tip/height"))
            .send()
            .await?;
        let block_tip_height = block_tip_height_response.text().await?;
        let block_tip_height = u32::from_str(block_tip_height.as_str())?;
        let block = block_tip_height + 1;
        tx.insert_target(block).await?;
        tx.commit().await?;
        Ok(Json(Target { block, nonce: None }))
    }
}

async fn post_target_block(
    Extension(state): Extension<Arc<State>>,
    block: String,
) -> Result<(), Error> {
    let new_block = u32::from_str(block.as_str())?;
    let mut tx = state.pool.begin().await?;
    let current_nonce = tx.select_current_target().await?.nonce;
    if current_nonce.is_some() {
        tx.insert_target(new_block).await?;
        tx.commit().await?;
        Ok(())
    } else {
        tx.rollback().await?;
        Err(Generic("Current target nonce is not set.".to_string()))
    }
}

async fn get_target_nonce(Extension(state): Extension<Arc<State>>) -> Result<String, Error> {
    //let nonce = u32::from_str(nonce.as_str())?;
    let client = reqwest::Client::new();
    let mut tx = state.pool.begin().await?;
    let target = tx.select_current_target().await?;
    if let Target { block, nonce: None } = target {
        let block_height_response = client
            .get(format!("https://mempool.space/api/block-height/{}", block))
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
                tx.commit().await?;
                return Ok(nonce.to_string());
            }
        }
    }
    Ok(target.nonce.map(|n| n.to_string()).unwrap_or_default())
}

async fn get_guesses(
    Extension(state): Extension<Arc<State>>,
    Path(block): Path<u32>,
) -> Result<Json<Vec<Guess>>, Error> {
    let mut tx = state.pool.begin().await?;
    let guesses = tx.select_guesses(block).await?;
    tx.commit().await?;
    Ok(Json(guesses))
}

async fn post_guess(
    Extension(state): Extension<Arc<State>>,
    Json(guess): Json<Guess>,
) -> Result<(), Error> {
    let mut tx = state.pool.begin().await?;
    tx.insert_guess(guess).await?;
    tx.commit().await?;
    Ok(())
}
