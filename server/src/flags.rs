//! Flags: learners report a problem with a question, which takes it out of
//! rotation for everyone until the flag is resolved.

use std::collections::HashSet;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use ts_rs::TS;

use crate::AppState;
use crate::api::ApiError;
use crate::auth::CurrentUser;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum FlagReason {
    WrongAnswer,
    Unclear,
    Typo,
    Other,
}

impl FlagReason {
    fn as_str(self) -> &'static str {
        match self {
            FlagReason::WrongAnswer => "wrong_answer",
            FlagReason::Unclear => "unclear",
            FlagReason::Typo => "typo",
            FlagReason::Other => "other",
        }
    }
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct FlagRequest {
    pub question_id: String,
    pub reason: FlagReason,
    /// Optional details, up to 500 characters.
    #[serde(default)]
    pub note: String,
}

pub const MAX_NOTE: usize = 500;

/// Ids of questions with at least one open flag.
pub async fn flagged_ids(pool: &SqlitePool) -> Result<HashSet<String>, ApiError> {
    let ids: Vec<String> = sqlx::query_scalar("SELECT DISTINCT question_id FROM flags WHERE resolved_at IS NULL")
        .fetch_all(pool)
        .await?;
    Ok(ids.into_iter().collect())
}

pub async fn create_flag(
    State(state): State<Arc<AppState>>,
    user: CurrentUser,
    Json(req): Json<FlagRequest>,
) -> Result<StatusCode, ApiError> {
    if !state.bank.questions.contains_key(&req.question_id) {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "No such question."));
    }
    let note: String = req.note.trim().chars().take(MAX_NOTE).collect();
    sqlx::query("INSERT INTO flags (user_id, question_id, content_hash, reason, note) VALUES (?, ?, ?, ?, ?)")
        .bind(user.id)
        .bind(&req.question_id)
        .bind(&state.bank.hashes[&req.question_id])
        .bind(req.reason.as_str())
        .bind(&note)
        .execute(&state.pool)
        .await?;
    tracing::info!("question {} flagged ({})", req.question_id, req.reason.as_str());
    Ok(StatusCode::NO_CONTENT)
}

/// An open flag, for the `gedpasser-server flags` command.
#[derive(Debug, sqlx::FromRow)]
pub struct OpenFlag {
    pub id: i64,
    pub question_id: String,
    pub reason: String,
    pub note: String,
    pub created_at: String,
    pub display_name: String,
}

pub async fn list_open(pool: &SqlitePool) -> sqlx::Result<Vec<OpenFlag>> {
    sqlx::query_as(
        "SELECT f.id, f.question_id, f.reason, f.note, f.created_at, u.display_name
         FROM flags f JOIN users u ON u.id = f.user_id
         WHERE f.resolved_at IS NULL ORDER BY f.id",
    )
    .fetch_all(pool)
    .await
}

/// Dismiss an open flag. Returns false if there was no open flag with that id.
pub async fn dismiss(pool: &SqlitePool, id: i64) -> sqlx::Result<bool> {
    let done = sqlx::query(
        "UPDATE flags SET resolved_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), resolution = 'dismissed'
         WHERE id = ? AND resolved_at IS NULL",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(done.rows_affected() == 1)
}
