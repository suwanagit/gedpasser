//! JSON API under `/api`. Every request/response type derives `TS`, so the
//! frontend's types in `web/src/api/types/` are generated, not hand-written.

use std::collections::{HashMap, HashSet};
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
use crate::figures::FigureView;
use crate::mastery::{Answer, Level, level, mastery};

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
            format!("Name must be 1 to {} characters.", auth::MAX_NAME_LEN),
        ));
    }
    if req.secret.chars().count() < auth::MIN_SECRET_LEN {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            format!("Password or PIN must be at least {} characters.", auth::MIN_SECRET_LEN),
        ));
    }
    let taken: Option<i64> = sqlx::query_scalar("SELECT id FROM users WHERE display_name = ?")
        .bind(&name)
        .fetch_optional(&state.pool)
        .await?;
    if taken.is_some() {
        return Err(ApiError::new(StatusCode::CONFLICT, "That name is already taken."));
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
        return Err(ApiError::new(StatusCode::UNAUTHORIZED, "Wrong password or PIN."));
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
    /// 0–1, weighted toward recent answers; `null` until the first answer.
    pub mastery: Option<f64>,
    pub level: Level,
}

/// A learner's answers grouped by skill and by principle, newest first.
pub(crate) struct LearnerStats {
    by_skill: HashMap<String, Vec<Answer>>,
    by_principle: HashMap<String, Vec<Answer>>,
}

impl LearnerStats {
    pub(crate) async fn load(state: &AppState, user_id: i64) -> Result<Self, ApiError> {
        let rows: Vec<(String, Option<String>, i64, i64)> = sqlx::query_as(
            "SELECT q.skill_id, a.principle_id, a.correct, a.hint_used
             FROM attempts a JOIN questions q ON q.id = a.question_id
             WHERE a.user_id = ? ORDER BY a.id DESC",
        )
        .bind(user_id)
        .fetch_all(&state.pool)
        .await?;
        let mut by_skill: HashMap<String, Vec<Answer>> = HashMap::new();
        let mut by_principle: HashMap<String, Vec<Answer>> = HashMap::new();
        for (skill, principle, correct, hint) in rows {
            let a = Answer { correct: correct != 0, hint_used: hint != 0 };
            by_skill.entry(skill).or_default().push(a);
            if let Some(p) = principle {
                by_principle.entry(p).or_default().push(a);
            }
        }
        Ok(LearnerStats { by_skill, by_principle })
    }

    /// (answered, correct, mastery or None, level) for a list of answers.
    fn summarize(answers: Option<&Vec<Answer>>) -> (u32, u32, Option<f64>, Level) {
        let answers = answers.map(Vec::as_slice).unwrap_or(&[]);
        let correct = answers.iter().filter(|a| a.correct).count();
        let m = mastery(answers);
        let shown = (!answers.is_empty()).then_some((m * 1000.0).round() / 1000.0);
        (answers.len() as u32, correct as u32, shown, level(answers.len(), m))
    }

    pub(crate) fn skill(&self, id: &str) -> (u32, u32, Option<f64>, Level) {
        Self::summarize(self.by_skill.get(id))
    }

    pub(crate) fn principle(&self, id: &str) -> (u32, u32, Option<f64>, Level) {
        Self::summarize(self.by_principle.get(id))
    }
}

/// Question ids from `ids` that can be served now (servable status, no open flag).
pub(crate) fn servable<'a>(state: &AppState, flagged: &HashSet<String>, ids: impl Iterator<Item = &'a String>) -> Vec<&'a String> {
    ids.filter(|id| !flagged.contains(*id))
        .filter(|id| state.bank.questions.get(*id).is_some_and(|q| Bank::servable(q, state.serve_drafts)))
        .collect()
}

pub(crate) fn principle_question_ids<'a>(state: &'a AppState, principle: &'a str) -> impl Iterator<Item = &'a String> + 'a {
    state.bank.questions.values().filter(move |q| q.principle == principle).map(|q| &q.id)
}

async fn skill_summaries(state: &AppState, user_id: i64) -> Result<Vec<SkillSummary>, ApiError> {
    let stats = LearnerStats::load(state, user_id).await?;
    let flagged = crate::flags::flagged_ids(&state.pool).await?;
    Ok(state
        .bank
        .skills
        .iter()
        .map(|s| {
            let ids = state.bank.by_skill.get(&s.id).map(Vec::as_slice).unwrap_or(&[]);
            let available = servable(state, &flagged, ids.iter()).len();
            let (answered, correct, mastery, level) = stats.skill(&s.id);
            SkillSummary {
                id: s.id.clone(),
                area: match s.area {
                    ged_content::Area::Quantitative => "quantitative".into(),
                    ged_content::Area::Algebraic => "algebraic".into(),
                },
                name: s.name.clone(),
                description: s.description.trim().to_string(),
                available: available as u32,
                answered,
                correct,
                mastery,
                level,
            }
        })
        .collect())
}

pub async fn list_skills(State(state): State<Arc<AppState>>, user: CurrentUser) -> ApiResult<Json<Vec<SkillSummary>>> {
    Ok(Json(skill_summaries(&state, user.id).await?))
}

/// The dashboard: every skill with mastery, plus the three to focus on next.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct Progress {
    pub skills: Vec<SkillSummary>,
    /// Up to three skill ids, weakest first.
    pub focus: Vec<String>,
}

/// The weakest skills that have questions: lowest mastery first (an untried
/// skill counts as 0.5), ties broken by fewer answers.
pub fn focus_skills(skills: &[SkillSummary], n: usize) -> Vec<String> {
    let mut candidates: Vec<&SkillSummary> = skills.iter().filter(|s| s.available > 0).collect();
    candidates.sort_by(|a, b| {
        let ma = a.mastery.unwrap_or(0.5);
        let mb = b.mastery.unwrap_or(0.5);
        ma.partial_cmp(&mb).unwrap_or(std::cmp::Ordering::Equal).then(a.answered.cmp(&b.answered))
    });
    candidates.into_iter().take(n).map(|s| s.id.clone()).collect()
}

pub async fn progress(State(state): State<Arc<AppState>>, user: CurrentUser) -> ApiResult<Json<Progress>> {
    let skills = skill_summaries(&state, user.id).await?;
    let focus = focus_skills(&skills, 3);
    Ok(Json(Progress { skills, focus }))
}

#[derive(Debug, Clone, Copy, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum QuestionKind {
    MultipleChoice,
    FillIn,
    Dropdown,
}

/// Which principle (official indicator) a question tests.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct PrincipleRef {
    /// Official indicator code, e.g. "Q.3.d".
    pub id: String,
    pub name: String,
}

pub(crate) fn principle_ref(state: &AppState, id: &str) -> PrincipleRef {
    let name = state.bank.principle(id).map(|p| p.name.clone()).unwrap_or_default();
    PrincipleRef { id: id.to_string(), name }
}

/// A question as shown to the learner: no answer, check or explanation.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct PracticeQuestion {
    pub id: String,
    pub skill: String,
    pub skill_name: String,
    pub principle: PrincipleRef,
    pub difficulty: u8,
    pub kind: QuestionKind,
    /// Text with math between `$...$`; a literal dollar sign is `\$`.
    pub prompt: String,
    /// Empty for fill-in questions.
    pub choices: Vec<String>,
    /// Graph, number line, chart or shape to show with the prompt.
    pub figure: Option<FigureView>,
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

/// What a practice session draws from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum PracticeMode {
    /// One skill.
    #[default]
    Skill,
    /// One principle (official indicator).
    Principle,
    /// Any skill, weighted toward the learner's weak ones.
    Mixed,
    /// Missed questions that are due again (spaced review).
    Review,
    /// A journey stage quiz.
    Journey,
}

impl PracticeMode {
    fn as_str(self) -> &'static str {
        match self {
            PracticeMode::Skill => "skill",
            PracticeMode::Principle => "principle",
            PracticeMode::Mixed => "mixed",
            PracticeMode::Review => "review",
            PracticeMode::Journey => "journey",
        }
    }
}

/// `GET /api/practice/next?skill=ID` | `?principle=ID[,ID…]` | `?mode=mixed`
/// | `?mode=review`, each with an optional `&after=ID` (the question just shown).
#[derive(Debug, Deserialize)]
pub struct NextParams {
    pub skill: Option<String>,
    pub principle: Option<String>,
    pub mode: Option<PracticeMode>,
    pub after: Option<String>,
}

pub(crate) fn random_u32() -> u32 {
    let mut b = [0u8; 4];
    let _ = getrandom::fill(&mut b);
    u32::from_le_bytes(b)
}

/// Pick a skill for mixed practice: weight (1 − mastery) + 0.1 over skills
/// that have questions, so weak skills come up most but none disappear.
pub fn pick_weighted(weights: &[(String, f64)], roll: f64) -> Option<String> {
    let total: f64 = weights.iter().map(|(_, w)| w).sum();
    if weights.is_empty() || total <= 0.0 {
        return None;
    }
    let mut target = roll.clamp(0.0, 0.999_999) * total;
    for (id, w) in weights {
        if target < *w {
            return Some(id.clone());
        }
        target -= w;
    }
    weights.last().map(|(id, _)| id.clone())
}

/// Pick the next question: never-answered first, then ones the learner got
/// wrong last time, then the rest, least recently seen first.
pub async fn next_question(
    State(state): State<Arc<AppState>>,
    user: CurrentUser,
    Query(params): Query<NextParams>,
) -> ApiResult<Json<NextQuestion>> {
    let flagged = crate::flags::flagged_ids(&state.pool).await?;
    if params.mode == Some(PracticeMode::Review) {
        // Soonest-due first; skip the question just shown if there's another.
        let due = crate::spaced::due_ids(&state.pool, user.id).await?;
        let due = servable(&state, &flagged, due.iter());
        let pick = due.iter().find(|id| Some(id.as_str()) != params.after.as_deref()).or(due.first());
        let question = match pick {
            Some(id) => Some(practice_question(&state, &state.bank.questions[*id])?),
            None => None,
        };
        return Ok(Json(NextQuestion { question }));
    }
    let mut pool: Vec<&String> = if let Some(skill) = &params.skill {
        if state.bank.skill(skill).is_none() {
            return Err(ApiError::new(StatusCode::NOT_FOUND, "No such skill."));
        }
        let ids = state.bank.by_skill.get(skill).map(Vec::as_slice).unwrap_or(&[]);
        servable(&state, &flagged, ids.iter())
    } else if let Some(list) = &params.principle {
        let mut ids = Vec::new();
        for principle in list.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            if state.bank.principle(principle).is_none() {
                return Err(ApiError::new(StatusCode::NOT_FOUND, "No such principle."));
            }
            ids.extend(servable(&state, &flagged, principle_question_ids(&state, principle)));
        }
        ids
    } else if params.mode == Some(PracticeMode::Mixed) {
        let stats = LearnerStats::load(&state, user.id).await?;
        let weights: Vec<(String, f64)> = state
            .bank
            .skills
            .iter()
            .filter(|s| {
                let ids = state.bank.by_skill.get(&s.id).map(Vec::as_slice).unwrap_or(&[]);
                !servable(&state, &flagged, ids.iter()).is_empty()
            })
            .map(|s| {
                let m = stats.skill(&s.id).2.unwrap_or(0.5);
                (s.id.clone(), (1.0 - m) + 0.1)
            })
            .collect();
        match pick_weighted(&weights, random_u32() as f64 / u32::MAX as f64) {
            Some(skill) => {
                let ids = state.bank.by_skill.get(&skill).map(Vec::as_slice).unwrap_or(&[]);
                servable(&state, &flagged, ids.iter())
            }
            None => Vec::new(),
        }
    } else {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "Say which skill, principle or mode=mixed."));
    };
    if pool.is_empty() {
        return Ok(Json(NextQuestion { question: None }));
    }

    // Per question: latest attempt id and whether that attempt was correct.
    let rows: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT a.question_id, a.id, a.correct FROM attempts a
         WHERE a.user_id = ?
           AND a.id = (SELECT MAX(id) FROM attempts b WHERE b.user_id = a.user_id AND b.question_id = a.question_id)",
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;
    let last: HashMap<String, (i64, bool)> = rows.into_iter().map(|(q, id, c)| (q, (id, c != 0))).collect();
    pool.sort(); // deterministic order before ranking

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
        tier[random_u32() as usize % tier.len()]
    } else {
        tier.sort_by_key(|id| rank(id).1); // least recently answered first
        tier[0]
    };

    Ok(Json(NextQuestion { question: Some(practice_question(&state, &state.bank.questions[chosen])?) }))
}

/// A question as the learner sees it (no answer key).
pub(crate) fn practice_question(state: &AppState, q: &ged_content::Question) -> Result<PracticeQuestion, ApiError> {
    let kind = match q.kind {
        QuestionType::MultipleChoice => QuestionKind::MultipleChoice,
        QuestionType::FillIn => QuestionKind::FillIn,
        QuestionType::Dropdown => QuestionKind::Dropdown,
        QuestionType::DragDrop => return Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "unsupported question type")),
    };
    Ok(PracticeQuestion {
        id: q.id.clone(),
        skill: q.skill.clone(),
        skill_name: state.bank.skill(&q.skill).map(|s| s.name.clone()).unwrap_or_default(),
        principle: principle_ref(state, &q.principle),
        difficulty: q.difficulty,
        kind,
        prompt: q.prompt.trim().to_string(),
        choices: q.choices.clone(),
        figure: q.figure.as_ref().map(FigureView::new),
        alaska: q.alaska,
        draft: q.status == ged_content::Status::Draft,
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct AnswerRequest {
    pub question_id: String,
    pub response: String,
    /// Milliseconds from showing the question to submitting.
    pub time_ms: u32,
    /// True if the learner opened the principle's lesson before answering.
    #[serde(default)]
    pub hint_used: bool,
    /// How the question was reached; stored with the attempt.
    #[serde(default)]
    pub mode: PracticeMode,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct AnswerResult {
    pub correct: bool,
    /// The correct answer, as written in the question bank.
    pub answer: String,
    pub explanation: String,
    pub common_mistake: String,
    pub principle: PrincipleRef,
    /// The principle's memory hack, if its card is available.
    pub hack: Option<String>,
    /// XP earned for this answer.
    pub xp: u32,
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
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "Please enter an answer."));
    }
    let response: String = req.response.chars().take(200).collect();
    let correct = grade(q, &response);
    sqlx::query(
        "INSERT INTO attempts
             (user_id, question_id, principle_id, response, correct, time_ms, mode, content_hash, hint_used)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(user.id)
    .bind(&q.id)
    .bind(&q.principle)
    .bind(&response)
    .bind(correct as i64)
    .bind(req.time_ms.min(24 * 60 * 60 * 1000) as i64)
    .bind(req.mode.as_str())
    .bind(&state.bank.hashes[&q.id])
    .bind(req.hint_used as i64)
    .execute(&state.pool)
    .await?;
    crate::spaced::record(&state.pool, user.id, &q.id, correct).await?;
    Ok(Json(AnswerResult {
        correct,
        answer: q.answer.clone(),
        explanation: q.explanation.trim().to_string(),
        common_mistake: q.common_mistake.trim().to_string(),
        principle: principle_ref(&state, &q.principle),
        hack: state.bank.card(&q.principle, state.serve_drafts).map(|c| c.hack.trim().to_string()),
        xp: crate::stats::answer_xp(correct),
    }))
}

// ── Principles and lessons ────────────────────────────────────────────────

/// A principle on the Lessons page, with this learner's progress on it.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct PrincipleSummary {
    /// Official indicator code, e.g. "Q.3.d".
    pub id: String,
    pub skill: String,
    pub name: String,
    pub summary: String,
    /// False when the lesson exists but isn't reviewed yet (and drafts are off).
    pub has_lesson: bool,
    /// Questions for this principle that can be served right now.
    pub available: u32,
    pub answered: u32,
    pub correct: u32,
    /// 0–1, weighted toward recent answers; `null` until the first answer.
    pub mastery: Option<f64>,
    pub level: Level,
}

/// A principle's full lesson, from `GET /api/principles/{id}`.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct Lesson {
    pub id: String,
    pub skill: String,
    pub skill_name: String,
    pub name: String,
    pub summary: String,
    /// Paragraphs separated by blank lines; "- " lines are bullets; math in
    /// `$...$`. `null` when the lesson isn't available yet.
    pub lesson: Option<String>,
    /// True if the lesson has not been reviewed yet (dev mode only).
    pub draft: bool,
    /// The Quick review card; the notes above are "more detail".
    pub card: Option<crate::journey::CardView>,
    /// Full lessons for this principle, one idea each.
    pub lessons: Vec<crate::lessons::LessonRef>,
}

/// Every principle, in skill order.
pub async fn list_principles(
    State(state): State<Arc<AppState>>,
    user: CurrentUser,
) -> ApiResult<Json<Vec<PrincipleSummary>>> {
    let stats = LearnerStats::load(&state, user.id).await?;
    let flagged = crate::flags::flagged_ids(&state.pool).await?;
    let out = state
        .bank
        .principles
        .iter()
        .map(|p| {
            let (answered, correct, mastery, level) = stats.principle(&p.id);
            PrincipleSummary {
                id: p.id.clone(),
                skill: p.skill.clone(),
                name: p.name.clone(),
                summary: p.summary.trim().to_string(),
                has_lesson: Bank::lesson_servable(p, state.serve_drafts),
                available: servable(&state, &flagged, principle_question_ids(&state, &p.id)).len() as u32,
                answered,
                correct,
                mastery,
                level,
            }
        })
        .collect();
    Ok(Json(out))
}

pub async fn get_principle(
    State(state): State<Arc<AppState>>,
    _user: CurrentUser,
    Path(id): Path<String>,
) -> ApiResult<Json<Lesson>> {
    let p = state
        .bank
        .principle(&id)
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "No such principle."))?;
    let show = Bank::lesson_servable(p, state.serve_drafts);
    Ok(Json(Lesson {
        id: p.id.clone(),
        skill: p.skill.clone(),
        skill_name: state.bank.skill(&p.skill).map(|s| s.name.clone()).unwrap_or_default(),
        name: p.name.clone(),
        summary: p.summary.trim().to_string(),
        lesson: show.then(|| p.lesson.trim().to_string()),
        draft: show && p.status == ged_content::Status::Draft,
        card: crate::journey::card_view(&state, &p.id),
        lessons: crate::lessons::lesson_refs(&state, &p.id),
    }))
}

pub async fn api_not_found() -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, "no such API route")
}
