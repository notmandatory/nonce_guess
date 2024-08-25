use axum::response::IntoResponse;
use clap::Parser;
use tracing::debug;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

// The handlers that process the data can be found in the auth.rs file
// This file contains the wasm client loading code and the axum routing
// use crate::auth::{
//     finish_authentication, finish_register, start_authentication, start_register, Backend, Error,
//     AUTH_UUID,
// };
use crate::web::App;

mod db;
mod error;
mod model;
// mod startup;
mod web;

/// Nonce Guess Server CLI arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    /// Address this server should listen on, defaults to "localhost:8081"
    #[arg(short, long, value_name = "HOST:PORT")]
    listen_address: Option<String>,
    #[arg(short, long, value_name = "DB_URL", env = "NONCE_GUESS_DB_URL")]
    /// SQLite DB URL for this server, ie. "sqlite://nonce_guess.sqlite?mode=rwc", defaults to in-memory DB
    database_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(EnvFilter::new(std::env::var("RUST_LOG").unwrap_or_else(
            |_| "axum_login=debug,tower_sessions=debug,sqlx=warn,tower_http=debug".into(),
        )))
        .with(tracing_subscriber::fmt::layer())
        .try_init()?;

    // TODO is "?mode=rwc" required? Yes
    // get database URL from env
    let database_url = std::env::var("NONCE_GUESS_DB_URL").ok();
    debug!("database_url: {:?}", &database_url);

    App::new(database_url).await?.serve().await
}
