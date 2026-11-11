//! JSON API under `/api`. Every request/response type derives `TS`, so the
//! frontend's types in `web/src/api/types/` are generated, not hand-written.

use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use ged_content::QuestionType;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::AppState;
use crate::auth::{self, CurrentUser};
use crate::content::{Bank, grade};

// ── Errors ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ApiErrorBody {
    pub error: String,
}

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        ApiError { status, message: message.into() }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(ApiErrorBody { error: self.message })).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        tracing::error!("database error: {e}");
        ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "database error")
    }
}

type ApiResult<T> = Result<T, ApiError>;

// ── Health ────────────────────────────────────────────────────────────────

/// Response of `GET /api/health`.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Health {
    pub status: String,
    pub version: String,
    /// Number of skills loaded from `content/skills.toml`.
    pub skills: u32,
}

pub async fn health(State(state): State<Arc<AppState>>) -> Json<Health> {
    Json(Health {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        skills: state.bank.skills.len() as u32,
    })
}

// ── Profiles and sessions ─────────────────────────────────────────────────

/// One entry in the profile picker.
#[derive(Debug, Serialize, TS, sqlx::FromRow)]
#[ts(export)]
pub struct Profile {
    #[ts(type = "number")]
    pub id: i64,
    pub display_name: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateProfile {
    pub display_name: String,
    /// Password or PIN, at least 4 characters.
    pub secret: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct LoginRequest {
    #[ts(type = "number")]
    pub user_id: i64,
    pub secret: String,
}

/// The logged-in learner, from `GET /api/me`, login and profile creation.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct Me {
    #[ts(type = "number")]
    pub id: i64,
    pub display_name: String,
    /// True when the server is serving draft questions (GED_SERVE_DRAFTS=1).
    pub serve_drafts: bool,
}

fn me(state: &AppState, id: i64, display_name: String) -> Me {
    Me { id, display_name, serve_drafts: state.serve_drafts }
}

pub async fn list_profiles(State(state): State<Arc<AppState>>) -> ApiResult<Json<Vec<Profile>>> {
    let rows = sqlx::query_as::<_, Profile>("SELECT id, display_name FROM users ORDER BY display_name COLLATE NOCASE")
        .fetch_all(&state.pool)
        .await?;
    Ok(Json(rows))
}

pub async fn create_profile(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateProfile>,
) -> ApiResult<(HeaderMap, Json<Me>)> {
    let name = req.display_name.trim().to_string();
    if name.is_empty() || name.chars().count() > auth::MAX_NAME_LEN {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            format!("name must be 1 to {} characters", auth::MAX_NAME_LEN),
        ));
    }
    if req.secret.chars().count() < auth::MIN_SECRET_LEN {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            format!("password or PIN must be at least {} characters", auth::MIN_SECRET_LEN),
        ));
    }
    let taken: Option<i64> = sqlx::query_scalar("SELECT id FROM users WHERE display_name = ?")
        .bind(&name)
        .fetch_optional(&state.pool)
        .await?;
    if taken.is_some() {
        return Err(ApiError::new(StatusCode::CONFLICT, "that name is already taken"));
    }
    let secret = req.secret;
    let hash = tokio::task::spawn_blocking(move || auth::hash_secret(&secret))
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "hashing failed"))?
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let id: i64 = sqlx::query_scalar("INSERT INTO users (display_name, password_hash) VALUES (?, ?) RETURNING id")
        .bind(&name)
        .bind(&hash)
        .fetch_one(&state.pool)
        .await?;
    tracing::info!("created profile {id}");
    let token = auth::start_session(&state, id).await?;
    Ok((auth::set_cookie_headers(auth::session_cookie(&token)), Json(me(&state, id, name))))
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> ApiResult<(HeaderMap, Json<Me>)> {
    let row: Option<(String, String)> = sqlx::query_as("SELECT display_name, password_hash FROM users WHERE id = ?")
        .bind(req.user_id)
        .fetch_optional(&state.pool)
        .await?;
    let stored = row.as_ref().map(|(_, h)| h.clone()).unwrap_or_else(|| auth::dummy_hash().to_string());
    let secret = req.secret;
    let ok = tokio::task::spawn_blocking(move || auth::verify_secret(&secret, &stored))
        .await
        .unwrap_or(false);
    let Some((name, _)) = row.filter(|_| ok) else {
        return Err(ApiError::new(StatusCode::UNAUTHORIZED, "wrong password or PIN"));
    };
    let token = auth::start_session(&state, req.user_id).await?;
    Ok((auth::set_cookie_headers(auth::session_cookie(&token)), Json(me(&state, req.user_id, name))))
}

pub async fn logout(State(state): State<Arc<AppState>>, headers: HeaderMap) -> ApiResult<(HeaderMap, StatusCode)> {
    if let Some(token) = auth::token_from_headers(&headers) {
        sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
            .bind(auth::token_hash(&token))
            .execute(&state.pool)
            .await?;
    }
    Ok((auth::set_cookie_headers(auth::clear_cookie()), StatusCode::NO_CONTENT))
}

pub async fn get_me(State(state): State<Arc<AppState>>, user: CurrentUser) -> Json<Me> {
    Json(me(&state, user.id, user.display_name))
}

// ── Skills and practice ───────────────────────────────────────────────────

/// A skill on the practice menu, with this learner's progress.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct SkillSummary {
    pub id: String,
    /// "quantitative" or "algebraic".
    pub area: String,
    pub name: String,
    pub description: String,
    /// Questions that can be served right now.
    pub available: u32,
    /// This learner's answers in this skill, all time.
    pub answered: u32,
    pub correct: u32,
}

pub async fn list_skills(State(state): State<Arc<AppState>>, user: CurrentUser) -> ApiResult<Json<Vec<SkillSummary>>> {
    let rows: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT q.skill_id, COUNT(*), COALESCE(SUM(a.correct), 0)
         FROM attempts a JOIN questions q ON q.id = a.question_id
         WHERE a.user_id = ? GROUP BY q.skill_id",
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;
    let stats: HashMap<String, (i64, i64)> = rows.into_iter().map(|(s, n, c)| (s, (n, c))).collect();
    let out = state
        .bank
        .skills
        .iter()
        .map(|s| {
            let available = state
                .bank
                .by_skill
                .get(&s.id)
                .map(|ids| ids.iter().filter(|id| Bank::servable(&state.bank.questions[*id], state.serve_drafts)).count())
                .unwrap_or(0);
            let (answered, correct) = stats.get(&s.id).copied().unwrap_or((0, 0));
            SkillSummary {
                id: s.id.clone(),
                area: match s.area {
                    ged_content::Area::Quantitative => "quantitative".into(),
                    ged_content::Area::Algebraic => "algebraic".into(),
                },
                name: s.name.clone(),
                description: s.description.trim().to_string(),
                available: available as u32,
                answered: answered as u32,
                correct: correct as u32,
            }
        })
        .collect();
    Ok(Json(out))
}

#[derive(Debug, Clone, Copy, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum QuestionKind {
    MultipleChoice,
    FillIn,
    Dropdown,
}

/// A question as shown to the learner: no answer, check or explanation.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct PracticeQuestion {
    pub id: String,
    pub skill: String,
    pub difficulty: u8,
    pub kind: QuestionKind,
    /// Text with math between `$...$`; a literal dollar sign is `\$`.
    pub prompt: String,
    /// Empty for fill-in questions.
    pub choices: Vec<String>,
    pub alaska: bool,
    /// True if the question has not been reviewed yet (dev mode only).
    pub draft: bool,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct NextQuestion {
    /// `null` when the skill has no questions available.
    pub question: Option<PracticeQuestion>,
}

#[derive(Debug, Deserialize)]
pub struct NextParams {
    /// The question just shown, so it is not repeated straight away.
    pub after: Option<String>,
}

/// Pick the next question for a skill: never-answered first, then ones the
/// learner got wrong last time, then the rest, least recently seen first.
pub async fn next_question(
    State(state): State<Arc<AppState>>,
    user: CurrentUser,
    Path(skill): Path<String>,
    Query(params): Query<NextParams>,
) -> ApiResult<Json<NextQuestion>> {
    if !state.bank.skills.iter().any(|s| s.id == skill) {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "no such skill"));
    }
    let ids = state.bank.by_skill.get(&skill).map(Vec::as_slice).unwrap_or(&[]);
    let mut pool: Vec<&String> = ids
        .iter()
        .filter(|id| Bank::servable(&state.bank.questions[*id], state.serve_drafts))
        .collect();
    if pool.is_empty() {
        return Ok(Json(NextQuestion { question: None }));
    }

    // Per question: latest attempt id and whether that attempt was correct.
    let rows: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT a.question_id, a.id, a.correct FROM attempts a
         JOIN questions q ON q.id = a.question_id
         WHERE a.user_id = ? AND q.skill_id = ?
           AND a.id = (SELECT MAX(id) FROM attempts b WHERE b.user_id = a.user_id AND b.question_id = a.question_id)",
    )
    .bind(user.id)
    .bind(&skill)
    .fetch_all(&state.pool)
    .await?;
    let last: HashMap<String, (i64, bool)> = rows.into_iter().map(|(q, id, c)| (q, (id, c != 0))).collect();

    if pool.len() > 1 {
        if let Some(after) = &params.after {
            pool.retain(|id| *id != after);
        }
    }
    // Rank: 0 = never answered, 1 = last answer wrong, 2 = last answer right.
    let rank = |id: &String| match last.get(id) {
        None => (0, 0),
        Some((attempt, false)) => (1, *attempt),
        Some((attempt, true)) => (2, *attempt),
    };
    let best = pool.iter().map(|id| rank(id).0).min().unwrap_or(0);
    let mut tier: Vec<&String> = pool.into_iter().filter(|id| rank(id).0 == best).collect();
    let chosen = if best == 0 {
        // Shuffle among unseen questions for variety.
        let mut b = [0u8; 4];
        let _ = getrandom::fill(&mut b);
        tier[u32::from_le_bytes(b) as usize % tier.len()]
    } else {
        tier.sort_by_key(|id| rank(id).1); // least recently answered first
        tier[0]
    };

    let q = &state.bank.questions[chosen];
    let kind = match q.kind {
        QuestionType::MultipleChoice => QuestionKind::MultipleChoice,
        QuestionType::FillIn => QuestionKind::FillIn,
        QuestionType::Dropdown => QuestionKind::Dropdown,
        QuestionType::DragDrop => return Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "unsupported question type")),
    };
    Ok(Json(NextQuestion {
        question: Some(PracticeQuestion {
            id: q.id.clone(),
            skill: q.skill.clone(),
            difficulty: q.difficulty,
            kind,
            prompt: q.prompt.trim().to_string(),
            choices: q.choices.clone(),
            alaska: q.alaska,
            draft: q.status == ged_content::Status::Draft,
        }),
    }))
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct AnswerRequest {
    pub question_id: String,
    pub response: String,
    /// Milliseconds from showing the question to submitting.
    pub time_ms: u32,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct AnswerResult {
    pub correct: bool,
    /// The correct answer, as written in the question bank.
    pub answer: String,
    pub explanation: String,
    pub common_mistake: String,
}

pub async fn submit_answer(
    State(state): State<Arc<AppState>>,
    user: CurrentUser,
    Json(req): Json<AnswerRequest>,
) -> ApiResult<Json<AnswerResult>> {
    let q = state
        .bank
        .questions
        .get(&req.question_id)
        .filter(|q| Bank::servable(q, state.serve_drafts))
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "no such question"))?;
    if req.response.trim().is_empty() {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "answer is empty"));
    }
    let response: String = req.response.chars().take(200).collect();
    let correct = grade(q, &response);
    sqlx::query(
        "INSERT INTO attempts (user_id, question_id, response, correct, time_ms, mode, content_hash)
         VALUES (?, ?, ?, ?, ?, 'practice', ?)",
    )
    .bind(user.id)
    .bind(&q.id)
    .bind(&response)
    .bind(correct as i64)
    .bind(req.time_ms.min(24 * 60 * 60 * 1000) as i64)
    .bind(&state.bank.hashes[&q.id])
    .execute(&state.pool)
    .await?;
    Ok(Json(AnswerResult {
        correct,
        answer: q.answer.clone(),
        explanation: q.explanation.trim().to_string(),
        common_mistake: q.common_mistake.trim().to_string(),
    }))
}

pub async fn api_not_found() -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, "no such API route")
}
