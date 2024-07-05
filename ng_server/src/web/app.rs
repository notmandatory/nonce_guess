use crate::web::auth::{Backend, Permission};
use crate::web::{auth, protected, restricted};
use axum::{Extension, Router};
use axum::extract::State;
use axum_embed::ServeEmbed;
use axum_login::{
    login_required, permission_required,
    tower_sessions::{ExpiredDeletion, Expiry, SessionManagerLayer},
    AuthManagerLayerBuilder,
};
use rust_embed::RustEmbed;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{migrate, Pool, Sqlite, SqlitePool};
use time::Duration;
use tokio::{signal, task::AbortHandle};
use tower_cookies::cookie::SameSite;
use tower_http::services::ServeDir;
use tower_sessions::cookie::Key;
use tower_sessions_sqlx_store::SqliteStore;

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

    pub async fn serve(self) -> Result<(), Box<dyn std::error::Error>> {
        // static assets
        let serve_assets = ServeEmbed::<Assets>::new();

        // Session layer.
        //
        // This uses `tower-sessions` to establish a layer that will provide the session
        // as a request extension.
        let session_store = SqliteStore::new(self.pool.clone());
        session_store.migrate().await?;

        // TODO add this back in
        // let deletion_task = tokio::task::spawn(
        //     session_store
        //         .clone()
        //         .continuously_delete_expired(tokio::time::Duration::from_secs(60)),
        // );

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
        let backend = Backend::new(self.pool.clone());
        let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

        let app =
            Router::new()
                .merge(restricted::router())
                .route_layer(permission_required!(Backend, login_url = "/login", Permission::ChangeTargetBlock))
                .merge(protected::router())
                .route_layer(login_required!(Backend, login_url = "/login"))
                .merge(auth::router())
                .layer(auth_layer)
                .with_state(self.pool)
                .nest_service("/assets", serve_assets);

        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

        // TODO add this back in
        // Ensure we use a shutdown signal to abort the deletion task.
        // axum::serve(listener, app.into_make_service())
        //     .with_graceful_shutdown(shutdown_signal(deletion_task.abort_handle()))
        //     .await?;
        //
        // deletion_task.await??;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

async fn shutdown_signal(deletion_task_abort_handle: AbortHandle) {
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
        _ = ctrl_c => { deletion_task_abort_handle.abort() },
        _ = terminate => { deletion_task_abort_handle.abort() },
    }
}
