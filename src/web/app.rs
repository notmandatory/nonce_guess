use crate::db::Db;
use crate::error::Error;
use crate::model::Block;
use crate::web::auth;
use crate::web::auth::Backend;
use axum::Router;
use axum_embed::ServeEmbed;
use axum_login::{
    login_required,
    tower_sessions::{Expiry, SessionManagerLayer},
    AuthManagerLayerBuilder,
};
use rust_embed::RustEmbed;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{migrate, Pool, Sqlite, SqlitePool};
use time::Duration;
use tokio::{signal, task::AbortHandle};
use tower_cookies::cookie::SameSite;
use tower_sessions::cookie::Key;
use tower_sessions::session_store::ExpiredDeletion;
use tower_sessions_sqlx_store::SqliteStore;
use tracing::error;

use super::{protected, restricted};

#[derive(RustEmbed, Clone)]
#[folder = "./assets"]
struct Assets;

pub struct App {
    pool: Pool<Sqlite>,
}

impl App {
    pub async fn new(database_url: Option<String>) -> Result<Self, Box<dyn std::error::Error>> {
        // setup connection pool
        let pool = if let Some(url) = database_url {
            SqlitePoolOptions::new().connect(&url).await?
        } else {
            // this should only be used for testing
            SqlitePoolOptions::new()
                .max_connections(1)
                .min_connections(1)
                .idle_timeout(None)
                .max_lifetime(None)
                .connect(":memory:")
                .await?
        };

        migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn serve(
        self,
        domain_name: String,
        web_url: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // static assets
        let serve_assets = ServeEmbed::<Assets>::new();

        // Session layer.
        //
        // This uses `tower-sessions` to establish a layer that will provide the session
        // as a request extension.
        let session_store = SqliteStore::new(self.pool.clone());
        session_store.migrate().await?;

        // task to update block hash when confirmed
        let update_task = tokio::task::spawn(continuously_update_target_nonce(self.pool.clone()));

        // task to delete expired sessions
        let delete_task = tokio::task::spawn(
            session_store
                .clone()
                .continuously_delete_expired(tokio::time::Duration::from_secs(360)),
        );

        // Generate a cryptographic key to sign the session cookie.
        let key = Key::generate();

        let session_layer = SessionManagerLayer::new(session_store)
            .with_name("webauthnrs")
            .with_same_site(SameSite::Strict)
            // TODO: change this to true when running on an HTTPS/production server instead of locally
            .with_secure(false)
            .with_expiry(Expiry::OnInactivity(Duration::hours(1)))
            .with_signed(key);

        // Auth service.
        //
        // This combines the session layer with our backend to establish the auth
        // service which will provide the auth session as a request extension.
        let backend = Backend::new(self.pool.clone(), domain_name, web_url);
        let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

        let app = Router::new()
            .merge(restricted::router())
            .merge(protected::router())
            .route_layer(login_required!(Backend, login_url = "/login"))
            .merge(auth::router())
            .layer(auth_layer)
            .with_state(self.pool)
            .nest_service("/assets", serve_assets);

        let listener = tokio::net::TcpListener::bind("0.0.0.0:8081").await.unwrap();

        // Ensure we use a shutdown signal to abort the tasks.
        axum::serve(listener, app.into_make_service())
            .with_graceful_shutdown(shutdown_signal(vec![
                update_task.abort_handle(),
                delete_task.abort_handle(),
            ]))
            .await?;
        update_task.await??;
        delete_task.await??;

        Ok(())
    }
}

async fn shutdown_signal(task_abort_handles: Vec<AbortHandle>) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => for abort_handle in task_abort_handles {
                abort_handle.abort()
            },
        _ = terminate => for abort_handle in task_abort_handles {
                abort_handle.abort()
            },
    }
}

async fn continuously_update_target_nonce(pool: SqlitePool) -> Result<(), Error> {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
    interval.tick().await; // The first tick completes immediately; skip.
    loop {
        interval.tick().await;
        if let Err(e) = update_target_nonce(pool.clone()).await {
            error!("update target error: {:?}", e);
        }
    }
}

async fn update_target_nonce(pool: SqlitePool) -> Result<(), Error> {
    let mut tx = pool.begin().await.map_err(crate::db::Error::Sqlx)?;
    let current_target = tx.select_current_target().await?;
    tx.commit().await.map_err(crate::db::Error::Sqlx)?;
    if current_target.nonce.is_none() {
        let client = reqwest::Client::builder()
            .use_native_tls()
            .danger_accept_invalid_certs(true)
            .build()?;
        let block_height_response = client
            .get(format!(
                "https://mempool.space/api/block-height/{}",
                current_target.block
            ))
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
                let mut tx = pool.begin().await.map_err(crate::db::Error::Sqlx)?;
                tx.set_target_nonce(current_target.block, nonce).await?;
                tx.set_guesses_block(current_target.block).await?;
                tx.commit().await.map_err(crate::db::Error::Sqlx)?;
            }
        }
    }
    Ok(())
}
