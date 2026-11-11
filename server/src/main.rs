//! GED Passer server: one binary that serves the JSON API under `/api` and
//! the built React app (embedded from `web/dist`) for every other path.
//!
//! Environment:
//!   GED_ADDR          listen address (default 0.0.0.0:8080, reachable on the LAN)
//!   GED_CONTENT       question bank directory (default: the nearest `content`
//!                     folder, see `find_content_dir`)
//!   GED_DB            SQLite file (default: `data/ged.db` beside `content`)
//!   GED_SERVE_DRAFTS  set to 1 to also serve draft questions (development only)

use std::io::IsTerminal;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use gedpasser_server::{Config, build_app};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_target(false).init();

    let content_dir = find_content_dir();
    let db_path = std::env::var("GED_DB").map(PathBuf::from).unwrap_or_else(|_| {
        let base = content_dir.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        base.join("data").join("ged.db")
    });
    let serve_drafts = matches!(std::env::var("GED_SERVE_DRAFTS").as_deref(), Ok("1" | "true" | "yes"));
    let config = Config { content_dir, db_path, serve_drafts };

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("flags") {
        std::process::exit(flags_command(&config, &args[1..]).await);
    }

    let app = build_app(&config).await.unwrap_or_else(|e| fatal(&e));
    tracing::info!(
        "loaded {} skills and {} questions from {}",
        app.skills,
        app.questions,
        config.content_dir.display()
    );
    tracing::info!("database: {}", config.db_path.display());
    if serve_drafts {
        tracing::warn!("GED_SERVE_DRAFTS is on: unreviewed draft questions will be served");
    }

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

    axum::serve(listener, app.router)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .expect("server error");
    app.pool.close().await;
}

/// `gedpasser-server flags` lists open flags; `flags resolve ID` dismisses one.
async fn flags_command(config: &Config, args: &[String]) -> i32 {
    let pool = match gedpasser_server::open_db(&config.db_path).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    let code = match (args.first().map(String::as_str), args.get(1)) {
        (None, _) => match gedpasser_server::flags::list_open(&pool).await {
            Ok(rows) if rows.is_empty() => {
                println!("No open flags.");
                0
            }
            Ok(rows) => {
                println!("{} open flag(s) in {}:", rows.len(), config.db_path.display());
                for f in rows {
                    let note = if f.note.is_empty() { String::new() } else { format!(" — {:?}", f.note) };
                    println!("  #{}  {}  {}  {} by {}{}", f.id, &f.created_at[..10], f.question_id, f.reason, f.display_name, note);
                }
                println!("Fix the question in content/ (its flag clears on restart), or: gedpasser-server flags resolve ID");
                0
            }
            Err(e) => {
                eprintln!("database error: {e}");
                1
            }
        },
        (Some("resolve"), Some(id)) => match id.trim_start_matches('#').parse::<i64>() {
            Ok(n) => match gedpasser_server::flags::dismiss(&pool, n).await {
                Ok(true) => {
                    println!("Flag #{n} dismissed. The question returns to practice.");
                    0
                }
                Ok(false) => {
                    eprintln!("No open flag #{n}.");
                    1
                }
                Err(e) => {
                    eprintln!("database error: {e}");
                    1
                }
            },
            Err(_) => {
                eprintln!("usage: gedpasser-server flags resolve ID");
                1
            }
        },
        _ => {
            eprintln!("usage: gedpasser-server flags [resolve ID]");
            1
        }
    };
    pool.close().await;
    code
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
