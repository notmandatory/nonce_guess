use axum::response::IntoResponse;
use clap::Parser;
use std::path::PathBuf;
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

//mod db;
mod error;
mod model;
// mod startup;
mod session_store;
pub mod web;

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
            |_| {
                format!(
                    "axum_login=debug,tower_sessions=debug,sqlx=warn,tower_http=debug,{}=debug",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            },
        )))
        .with(tracing_subscriber::fmt::layer())
        .try_init()?;

    // get database URL from env
    let database_file = std::env::var("NONCE_GUESS_DB_FILE")
        .ok()
        .map(|path| PathBuf::from(path));
    debug!("database_file: {:?}", &database_file);

    // get effective public domainname from env
    let domain_name = std::env::var("NONCE_GUESS_DOMAIN_NAME").unwrap_or("localhost".to_string());
    debug!("domain_name: {:?}", &domain_name);

    // get effective public web service URL from env
    let web_url =
        std::env::var("NONCE_GUESS_WEB_URL").unwrap_or("http://localhost:8081".to_string());
    debug!("web_url: {:?}", &web_url);

    App::new(database_file)
        .await?
        .serve(domain_name, web_url)
        .await
}
