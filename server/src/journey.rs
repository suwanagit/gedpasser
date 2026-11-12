//! The journey: a guided path of stages. Each stage teaches a few principles
//! with short cards, then a short quiz; passing unlocks the next stage.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Serialize;
use ts_rs::TS;

use crate::AppState;
use crate::api::{ApiError, PracticeQuestion, PrincipleRef, SubjectParam, practice_question, principle_ref, random_u32, servable};
use crate::auth::CurrentUser;
use crate::figures::FigureView;
use crate::spaced::now_unix;
use crate::stats::stage_xp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    Locked,
    Open,
    Done,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct StageView {
    pub id: String,
    /// Day of the week path, 1–7.
    pub day: u8,
    pub title: String,
    pub checkpoint: bool,
    /// Principles taught here; for a checkpoint, the principles it mixes.
    pub principles: Vec<PrincipleRef>,
    pub size: u32,
    pub status: StageStatus,
    /// 0 until passed, then 1–3.
    pub stars: u32,
    pub best_correct: Option<u32>,
    pub best_total: Option<u32>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct JourneyView {
    pub stages: Vec<StageView>,
    /// The first open, not-yet-passed stage.
    pub current: Option<String>,
    pub passed: u32,
}

/// A principle's short lesson card.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct CardView {
    pub principle: PrincipleRef,
    pub skill_name: String,
    pub title: String,
    /// The one rule; may contain $$display math$$.
    pub key: String,
    pub steps: Vec<String>,
    pub example: String,
    /// Memory device or test shortcut.
    pub hack: String,
    pub figure: Option<FigureView>,
    pub draft: bool,
}

pub(crate) fn card_view(state: &AppState, principle: &str) -> Option<CardView> {
    let c = state.bank.card(principle, state.serve_drafts)?;
    let p = state.bank.principle(principle)?;
    Some(CardView {
        principle: principle_ref(state, principle),
        skill_name: state.bank.skill(&p.skill).map(|s| s.name.clone()).unwrap_or_default(),
        title: c.title.clone(),
        key: c.key.trim().to_string(),
        steps: c.steps.clone(),
        example: c.example.trim().to_string(),
        hack: c.hack.trim().to_string(),
        figure: c.figure.as_ref().map(FigureView::new),
        draft: c.status == ged_content::Status::Draft,
    })
}

/// Principles a stage covers: its own, or for a checkpoint, those taught
/// since the previous checkpoint (all so far if cumulative).
fn stage_principles(state: &AppState, index: usize) -> Vec<String> {
    let stages = &state.bank.journey;
    let s = &stages[index];
    if !s.checkpoint {
        return s.principles.clone();
    }
    let mut out = Vec::new();
    for prev in stages[..index].iter().rev().filter(|p| p.subject == s.subject) {
        if prev.checkpoint && !s.cumulative {
            break;
        }
        out.extend(prev.principles.iter().cloned());
    }
    out.reverse();
    out
}

async fn progress(state: &AppState, user_id: i64) -> Result<HashMap<String, (u32, u32, u32)>, ApiError> {
    let rows: Vec<(String, i64, i64, i64)> =
        sqlx::query_as("SELECT stage_id, best_correct, best_total, stars FROM journey_progress WHERE user_id = ?")
            .bind(user_id)
            .fetch_all(&state.pool)
            .await?;
    Ok(rows.into_iter().map(|(s, c, t, st)| (s, (c as u32, t as u32, st as u32))).collect())
}

/// One subject's week path with this learner's progress.
pub(crate) async fn build_view(state: &AppState, user_id: i64, subject: &str) -> Result<JourneyView, ApiError> {
    let done = progress(state, user_id).await?;
    let mut stages = Vec::new();
    let mut prev_passed = true;
    let mut current = None;
    let mut passed = 0;
    for (i, s) in state.bank.path(subject) {
        let best = done.get(&s.id);
        let stars = best.map(|b| b.2).unwrap_or(0);
        let status = if stars > 0 {
            StageStatus::Done
        } else if prev_passed {
            StageStatus::Open
        } else {
            StageStatus::Locked
        };
        if stars > 0 {
            passed += 1;
        }
        if status == StageStatus::Open && current.is_none() {
            current = Some(s.id.clone());
        }
        prev_passed = stars > 0;
        stages.push(StageView {
            id: s.id.clone(),
            day: s.day,
            title: s.title.clone(),
            checkpoint: s.checkpoint,
            principles: stage_principles(state, i).iter().map(|p| principle_ref(state, p)).collect(),
            size: s.size,
            status,
            stars,
            best_correct: best.map(|b| b.0),
            best_total: best.map(|b| b.1),
        });
    }
    Ok(JourneyView { stages, current, passed })
}

pub async fn get_journey(
    State(state): State<Arc<AppState>>,
    user: CurrentUser,
    Query(sp): Query<SubjectParam>,
) -> Result<Json<JourneyView>, ApiError> {
    let subject = sp.resolve(&state)?;
    Ok(Json(build_view(&state, user.id, subject).await?))
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct JourneyRun {
    #[ts(type = "number")]
    pub run_id: i64,
    pub stage: StageView,
    /// Cards for the stage's principles (none for checkpoints).
    pub cards: Vec<CardView>,
    pub questions: Vec<PracticeQuestion>,
}

/// Choose quiz questions: spread across the principles, preferring ones at
/// or below the stage's difficulty, easiest first.
fn pick_questions(state: &AppState, principles: &[String], size: usize, max_difficulty: u8, flagged: &HashSet<String>) -> Vec<String> {
    let mut groups: Vec<Vec<String>> = principles
        .iter()
        .map(|p| {
            let ids: Vec<&String> =
                state.bank.questions.values().filter(|q| &q.principle == p).map(|q| &q.id).collect();
            let mut ids: Vec<String> = servable(state, flagged, ids.into_iter()).into_iter().cloned().collect();
            // Shuffle, then put questions within the difficulty cap first.
            for i in (1..ids.len()).rev() {
                ids.swap(i, random_u32() as usize % (i + 1));
            }
            ids.sort_by_key(|id| state.bank.questions[id].difficulty > max_difficulty);
            ids
        })
        .filter(|g| !g.is_empty())
        .collect();
    let mut chosen = Vec::new();
    let mut round = 0;
    while chosen.len() < size && groups.iter().any(|g| g.len() > round) {
        for g in &mut groups {
            if chosen.len() < size && g.len() > round {
                chosen.push(g[round].clone());
            }
        }
        round += 1;
    }
    chosen.sort_by_key(|id| state.bank.questions[id].difficulty);
    chosen
}

pub async fn start_stage(
    State(state): State<Arc<AppState>>,
    user: CurrentUser,
    Path(id): Path<String>,
) -> Result<Json<JourneyRun>, ApiError> {
    let index = state
        .bank
        .journey
        .iter()
        .position(|s| s.id == id)
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "No such stage."))?;
    let subject = state.bank.journey[index].subject.clone();
    let view = build_view(&state, user.id, &subject).await?;
    let stage_view = view
        .stages
        .iter()
        .find(|v| v.id == id)
        .cloned()
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "No such stage."))?;
    if stage_view.status == StageStatus::Locked {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "Finish the stage before this one first."));
    }
    let s = &state.bank.journey[index];
    let principles = stage_principles(&state, index);
    let flagged = crate::flags::flagged_ids(&state.pool).await?;
    let ids = pick_questions(&state, &principles, s.size as usize, s.max_difficulty, &flagged);
    if ids.is_empty() {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "No questions are available for this stage yet."));
    }
    let run_id: i64 = sqlx::query_scalar(
        "INSERT INTO journey_runs (user_id, stage_id, question_ids, started_at) VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind(user.id)
    .bind(&s.id)
    .bind(serde_json::to_string(&ids).unwrap_or_default())
    .bind(now_unix())
    .fetch_one(&state.pool)
    .await?;
    let cards = if s.checkpoint { Vec::new() } else { principles.iter().filter_map(|p| card_view(&state, p)).collect() };
    let questions = ids
        .iter()
        .map(|id| practice_question(&state, &state.bank.questions[id]))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(JourneyRun { run_id, stage: stage_view, cards, questions }))
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct HackItem {
    pub principle: PrincipleRef,
    pub title: String,
    pub hack: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct StageResult {
    pub correct: u32,
    pub total: u32,
    pub passed: bool,
    pub stars: u32,
    /// Better than any earlier attempt at this stage.
    pub improved: bool,
    /// XP from the stage bonus (answer XP is shown per answer).
    pub bonus_xp: u32,
    pub next_stage: Option<String>,
    pub missed: Vec<PrincipleRef>,
    /// Hacks for the principles missed (or all of the stage's, if none).
    pub hacks: Vec<HackItem>,
}

/// Stars for a score: pass at about two thirds, 2 stars at 80%, 3 at 100%.
pub fn stars_for(correct: u32, total: u32) -> u32 {
    if total == 0 || correct * 3 < total * 2 {
        0
    } else if correct == total {
        3
    } else if correct * 5 >= total * 4 {
        2
    } else {
        1
    }
}

pub(crate) fn hack_item(state: &AppState, principle: &str) -> Option<HackItem> {
    let c = state.bank.card(principle, state.serve_drafts)?;
    Some(HackItem { principle: principle_ref(state, principle), title: c.title.clone(), hack: c.hack.trim().to_string() })
}

pub async fn finish_stage(
    State(state): State<Arc<AppState>>,
    user: CurrentUser,
    Path(run_id): Path<i64>,
) -> Result<Json<StageResult>, ApiError> {
    let run: Option<(String, String, i64, Option<i64>)> = sqlx::query_as(
        "SELECT stage_id, question_ids, started_at, finished_at FROM journey_runs WHERE id = ? AND user_id = ?",
    )
    .bind(run_id)
    .bind(user.id)
    .fetch_optional(&state.pool)
    .await?;
    let (stage_id, ids_json, started_at, finished) =
        run.ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "No such run."))?;
    if finished.is_some() {
        return Err(ApiError::new(StatusCode::CONFLICT, "This run is already finished."));
    }
    let ids: Vec<String> = serde_json::from_str(&ids_json).unwrap_or_default();
    // Each question's latest journey answer since the run started.
    let mut correct = 0u32;
    let mut missed_principles = Vec::new();
    for id in &ids {
        let c: Option<i64> = sqlx::query_scalar(
            "SELECT correct FROM attempts WHERE user_id = ? AND question_id = ? AND mode = 'journey'
               AND CAST(strftime('%s', answered_at) AS INTEGER) >= ? ORDER BY id DESC LIMIT 1",
        )
        .bind(user.id)
        .bind(id)
        .bind(started_at)
        .fetch_optional(&state.pool)
        .await?;
        if c == Some(1) {
            correct += 1;
        } else if let Some(q) = state.bank.questions.get(id) {
            if !missed_principles.contains(&q.principle) {
                missed_principles.push(q.principle.clone());
            }
        }
    }
    let total = ids.len() as u32;
    let stars = stars_for(correct, total);
    let now = now_unix();
    sqlx::query("UPDATE journey_runs SET finished_at = ?, correct = ?, total = ? WHERE id = ?")
        .bind(now)
        .bind(correct as i64)
        .bind(total as i64)
        .bind(run_id)
        .execute(&state.pool)
        .await?;

    let prev: Option<(i64, i64, i64)> = sqlx::query_as(
        "SELECT best_correct, best_total, stars FROM journey_progress WHERE user_id = ? AND stage_id = ?",
    )
    .bind(user.id)
    .bind(&stage_id)
    .fetch_optional(&state.pool)
    .await?;
    let prev_stars = prev.map(|p| p.2 as u32).unwrap_or(0);
    let better = match prev {
        None => true,
        Some((pc, pt, ps)) => stars > ps as u32 || (stars == ps as u32 && (correct as i64) * pt > pc * total as i64),
    };
    if better {
        sqlx::query(
            "INSERT INTO journey_progress (user_id, stage_id, best_correct, best_total, stars, completed_at)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(user_id, stage_id) DO UPDATE SET best_correct = excluded.best_correct,
                 best_total = excluded.best_total, stars = excluded.stars, completed_at = excluded.completed_at",
        )
        .bind(user.id)
        .bind(&stage_id)
        .bind(correct as i64)
        .bind(total as i64)
        .bind(stars.max(prev_stars) as i64)
        .bind(now)
        .execute(&state.pool)
        .await?;
    }
    let bonus_xp = stage_xp(stars).saturating_sub(stage_xp(prev_stars));

    let index = state.bank.journey.iter().position(|s| s.id == stage_id).unwrap_or(0);
    let subject = &state.bank.journey[index].subject;
    let next_stage = if stars > 0 {
        state.bank.journey[index + 1..].iter().find(|s| &s.subject == subject).map(|s| s.id.clone())
    } else {
        None
    };
    let hack_from: Vec<String> =
        if missed_principles.is_empty() { stage_principles(&state, index) } else { missed_principles.clone() };
    Ok(Json(StageResult {
        correct,
        total,
        passed: stars > 0,
        stars,
        improved: stars > prev_stars,
        bonus_xp,
        next_stage,
        missed: missed_principles.iter().map(|p| principle_ref(&state, p)).collect(),
        hacks: hack_from.iter().filter_map(|p| hack_item(&state, p)).take(4).collect(),
    }))
}

#[cfg(test)]
mod tests {
    use super::stars_for;

    #[test]
    fn stars() {
        assert_eq!(stars_for(1, 3), 0);
        assert_eq!(stars_for(2, 3), 1);
        assert_eq!(stars_for(3, 3), 3);
        assert_eq!(stars_for(4, 5), 2);
        assert_eq!(stars_for(7, 10), 1);
        assert_eq!(stars_for(0, 0), 0);
    }
}
