//! GED Passer server library: builds the Axum app (JSON API under `/api`,
//! embedded React app for everything else) on top of SQLite.

pub mod api;
pub mod auth;
pub mod content;
pub mod flags;

use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use rust_embed::RustEmbed;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

use crate::content::Bank;

pub struct Config {
    pub content_dir: PathBuf,
    pub db_path: PathBuf,
    /// Also serve `draft` questions (GED_SERVE_DRAFTS=1). Development only.
    pub serve_drafts: bool,
}

pub struct AppState {
    pub pool: SqlitePool,
    pub bank: Bank,
    pub serve_drafts: bool,
}

pub struct App {
    pub router: Router,
    pub pool: SqlitePool,
    pub skills: usize,
    pub questions: usize,
}

/// Open (creating if needed) the SQLite file and run migrations.
pub async fn open_db(path: &Path) -> Result<SqlitePool, String> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    let opts = SqliteConnectOptions::from_str("sqlite:")
        .map_err(|e| e.to_string())?
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await
        .map_err(|e| format!("cannot open database {}: {e}", path.display()))?;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| format!("database migration failed: {e}"))?;
    Ok(pool)
}

/// Load the question bank, open the database, sync, and build the router.
pub async fn build_app(config: &Config) -> Result<App, String> {
    let bank = Bank::load(&config.content_dir)?;
    let pool = open_db(&config.db_path).await?;
    bank.sync(&pool).await.map_err(|e| format!("cannot sync question bank into the database: {e}"))?;
    let (skills, questions) = (bank.skills.len(), bank.questions.len());
    let state = Arc::new(AppState { pool: pool.clone(), bank, serve_drafts: config.serve_drafts });

    let router = Router::new()
        .route("/api/health", get(api::health))
        .route("/api/profiles", get(api::list_profiles).post(api::create_profile))
        .route("/api/login", post(api::login))
        .route("/api/logout", post(api::logout))
        .route("/api/me", get(api::get_me))
        .route("/api/skills", get(api::list_skills))
        .route("/api/practice/{skill}/next", get(api::next_question))
        .route("/api/answer", post(api::submit_answer))
        .route("/api/principles", get(api::list_principles))
        .route("/api/principles/{id}", get(api::get_principle))
        .route("/api/flags", post(flags::create_flag))
        .route("/api/{*rest}", get(api::api_not_found).post(api::api_not_found))
        .fallback(static_file)
        .with_state(state);
    Ok(App { router, pool, skills, questions })
}

#[derive(RustEmbed)]
#[folder = "../web/dist"]
struct Assets;

/// Serve a file from the embedded frontend. Unknown paths get `index.html`
/// so client-side routes work on reload.
async fn static_file(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    match Assets::get(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            // Vite puts content-hashed files under assets/, so they never change.
            let cache = if path.starts_with("assets/") { "public, max-age=31536000, immutable" } else { "no-cache" };
            ([(header::CONTENT_TYPE, mime.as_ref()), (header::CACHE_CONTROL, cache)], file.data).into_response()
        }
        None => match Assets::get("index.html") {
            Some(index) => {
                ([(header::CONTENT_TYPE, "text/html; charset=utf-8"), (header::CACHE_CONTROL, "no-cache")], index.data)
                    .into_response()
            }
            None => (StatusCode::NOT_FOUND, "frontend not built: run scripts/build.ps1").into_response(),
        },
    }
}
