mod auth;
mod config;
mod db;
mod error;
mod routes;
mod templates;

use std::time::Duration;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::SqlitePool;
use tower_http::trace::TraceLayer;
use tower_sessions::{session_store::ExpiredDeletion, Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;

use config::Config;

#[derive(Clone)]
struct AppState {
    pool: SqlitePool,
    config: std::sync::Arc<Config>,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,tower_http=debug")),
        )
        .init();

    let config = Config::from_env();
    let bind_addr = config.bind_addr;
    tracing::info!(
        user_tz = %config.user_tz,
        display_mass_unit = ?config.display_mass_unit,
        "configuration loaded"
    );

    let pool = db::connect(&config.database_url)
        .await
        .expect("failed to connect to database");
    db::migrate(&pool).await.expect("failed to run migrations");

    let session_store = SqliteStore::new(pool.clone());
    session_store
        .migrate()
        .await
        .expect("failed to run session store migrations");

    let deletion_task = tokio::task::spawn(
        session_store
            .clone()
            .continuously_delete_expired(Duration::from_secs(60)),
    );

    let session_layer = SessionManagerLayer::new(session_store)
        .with_signed(config.session_key.clone())
        .with_secure(false) // set true once served over HTTPS
        .with_expiry(Expiry::OnInactivity(time::Duration::days(30)));

    let state = AppState {
        pool,
        config: std::sync::Arc::new(config),
    };

    let app = Router::new()
        .route("/health", get(routes::health::health))
        .route("/", get(routes::home::home))
        .route("/login", get(auth::login_page).post(auth::login_submit))
        .route("/logout", post(auth::logout))
        .nest_service("/static", tower_http::services::ServeDir::new("static"))
        .layer(session_layer)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .expect("failed to bind listener");
    tracing::info!(%bind_addr, "listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");

    deletion_task.abort();
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
    tracing::info!("shutting down");
}
