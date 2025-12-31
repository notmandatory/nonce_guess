use crate::auth::backend::AuthBackend;
use crate::guess::backend::{continuously_update_target_nonce, GuessBackend};
use crate::session_store::RedbSessionStore;
use crate::{auth, guess};
use axum::Router;
use axum_embed::ServeEmbed;
use axum_login::{
    tower_sessions::{Expiry, SessionManagerLayer},
    AuthManagerLayerBuilder,
};
use redb::Database;
use reqwest::Url;
use rust_embed::RustEmbed;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::NamedTempFile;
use time::Duration;
use tokio::{signal, task::AbortHandle};
use tower_cookies::cookie::SameSite;
use tower_sessions::cookie::Key;

#[derive(RustEmbed, Clone)]
#[folder = "assets/"]
struct Assets;

pub struct App {
    db: Arc<Database>,
    http_client: reqwest::Client,
    mempool_url: Url,
}

pub struct AppState {
    pub guess_backend: Arc<GuessBackend>,
}

impl App {
    pub async fn new(
        database_file: Option<PathBuf>,
        mempool_url: Option<Url>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // setup database file
        let db = if let Some(file) = database_file {
            Database::create(file)?
        } else {
            // temp file should only be used for testing
            let file = NamedTempFile::new()?.into_temp_path();
            Database::create(file)?
        };
        let http_client = reqwest::Client::builder()
            .use_native_tls()
            .danger_accept_invalid_certs(true)
            .build()?;
        let mempool_url = mempool_url.unwrap_or(Url::parse("https://mempool.space")?);

        // TODO: call database migrations here

        Ok(Self {
            db: Arc::new(db),
            http_client,
            mempool_url,
        })
    }

    pub async fn serve(self) -> Result<(), Box<dyn std::error::Error>> {
        // static assets
        let serve_assets = ServeEmbed::<Assets>::new();

        // Session layer.
        //
        // This uses `tower-sessions` to establish a layer that will provide the session
        // as a request extension.
        let session_store = RedbSessionStore::new(self.db.clone());

        // task to delete expired sessions
        let delete_task = tokio::task::spawn(
            session_store
                .clone()
                .continuously_delete_expired(tokio::time::Duration::from_secs(360)),
        );

        // Generate a cryptographic key to sign the session cookie.
        let key = Key::generate();

        let session_layer = SessionManagerLayer::new(session_store)
            .with_same_site(SameSite::Strict)
            // TODO: change this to true when running on an HTTPS/production server instead of locally
            .with_secure(false)
            .with_expiry(Expiry::OnInactivity(Duration::hours(1)))
            .with_signed(key);

        // Auth service.
        //
        // This combines the session layer with our backend to establish the auth
        // service which will provide the auth session as a request extension.
        let auth_backend = AuthBackend::new(self.db.clone())?;
        let auth_layer = AuthManagerLayerBuilder::new(auth_backend, session_layer).build();

        let guess_backend = GuessBackend::new(
            self.db.clone(),
            self.http_client.clone(),
            self.mempool_url.clone(),
        )
        .map(Arc::new)?;

        // task to update block hash when confirmed
        let update_task =
            tokio::task::spawn(continuously_update_target_nonce(guess_backend.clone()));

        let app_state = Arc::new(AppState { guess_backend });

        let router = Router::new()
            .merge(auth::web::router())
            .merge(guess::web::router())
            .layer(auth_layer)
            .with_state(app_state)
            .nest_service("/assets", serve_assets);

        let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();

        // Ensure we use a shutdown signal to abort the tasks.
        axum::serve(listener, router.into_make_service())
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
