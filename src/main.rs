use crate::app::App;
use std::path::PathBuf;
use tracing::debug;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

pub mod app;
pub mod auth;
pub mod guess;
mod session_store;
mod types;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(EnvFilter::new(std::env::var("RUST_LOG").unwrap_or_else(
            |_| {
                format!(
                    "axum_login=debug,tower_sessions=debug,sqlx=warn,tower_http=debug,{}=debug",
                    env!("CARGO_CRATE_NAME")
                )
            },
        )))
        .with(tracing_subscriber::fmt::layer())
        .try_init()?;

    // get database file name from env
    let database_file = std::env::var("NONCE_GUESS_DB_FILE").ok().map(PathBuf::from);
    debug!("database_file: {:?}", &database_file);

    App::new(database_file)
        .await?
        .serve()
        .await
}
