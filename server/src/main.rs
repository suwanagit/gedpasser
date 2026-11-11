//! GED Passer server: one binary that serves the JSON API under `/api` and
//! the built React app (embedded from `web/dist`) for every other path.
//!
//! Environment:
//!   GED_ADDR     listen address (default 0.0.0.0:8080, reachable on the LAN)
//!   GED_CONTENT  question bank directory (default: the nearest `content`
//!                folder, see `find_content_dir`)

use std::io::IsTerminal;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use rust_embed::RustEmbed;
use serde::Serialize;
use ts_rs::TS;

#[derive(RustEmbed)]
#[folder = "../web/dist"]
struct Assets;

/// Response of `GET /api/health`.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Health {
    pub status: String,
    pub version: String,
    /// Number of skills loaded from `content/skills.toml`.
    pub skills: u32,
}

struct AppState {
    skills: u32,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_target(false).init();

    let content = find_content_dir();
    let skills = match ged_content::load_skills(&content) {
        Ok(s) => s,
        Err(e) => fatal(&format!("cannot load skill map from {}:\n{e}", content.display())),
    };
    tracing::info!("loaded {} skills from {}", skills.len(), content.display());

    let state = Arc::new(AppState { skills: skills.len() as u32 });
    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/{*rest}", get(api_not_found))
        .fallback(static_file)
        .with_state(state);

    let addr_text = std::env::var("GED_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let addr: SocketAddr = addr_text
        .parse()
        .unwrap_or_else(|_| fatal(&format!("GED_ADDR {addr_text:?} must look like 0.0.0.0:8080")));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| fatal(&format!("cannot listen on {addr}: {e}")));
    tracing::info!("listening on {addr}");
    println!("\nGED Passer is running. Open http://localhost:{} in a browser.", addr.port());
    println!("Other devices on your network: http://<this PC's IP address>:{}", addr.port());
    println!("Press Ctrl+C or close this window to stop.\n");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .expect("server error");
}

/// Find the question bank: `GED_CONTENT` if set, else a `content` folder
/// with a `skills.toml` in the current directory, the executable's folder,
/// or any parent of it (so `target\release\gedpasser-server.exe` finds the
/// repo's `content` when double-clicked).
fn find_content_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("GED_CONTENT") {
        return PathBuf::from(dir);
    }
    let has_skills = |dir: &Path| dir.join("content").join("skills.toml").is_file();
    let cwd = std::env::current_dir().ok();
    if let Some(cwd) = cwd.as_deref().filter(|d| has_skills(d)) {
        return cwd.join("content");
    }
    if let Some(exe_dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
        if let Some(found) = exe_dir.ancestors().find(|d| has_skills(d)) {
            return found.join("content");
        }
    }
    PathBuf::from("content")
}

/// Print an error and exit. When run from a console window (for example a
/// double-click), wait for Enter first so the message can be read.
fn fatal(msg: &str) -> ! {
    eprintln!("\nERROR: {msg}");
    if std::io::stdin().is_terminal() {
        eprintln!("\nPress Enter to close.");
        let _ = std::io::stdin().read_line(&mut String::new());
    }
    std::process::exit(1);
}

async fn health(State(state): State<Arc<AppState>>) -> Json<Health> {
    Json(Health {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        skills: state.skills,
    })
}

async fn api_not_found() -> Response {
    (StatusCode::NOT_FOUND, "no such API route").into_response()
}

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
