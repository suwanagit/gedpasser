//! Test-week plan: most learners start a week before the test and won't
//! master all 62 principles. Rank principles by how often they come up
//! (yield) × how weak the learner is, and lay out today's tasks by days left.
//! Also serves the strategy content: tactics, formula sheet, all hacks.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::AppState;
use crate::api::{ApiError, LearnerStats, PrincipleRef, principle_question_ids, principle_ref, servable};
use crate::auth::CurrentUser;
use crate::journey::{HackItem, hack_item};
use crate::mastery::Level;
use crate::spaced::now_unix;
use crate::stats::TzParams;

/// Days since 1970-01-01 for a civil date (Howard Hinnant's algorithm).
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

pub fn parse_date(s: &str) -> Option<i64> {
    let mut it = s.trim().split('-');
    let (y, m, d) = (it.next()?.parse().ok()?, it.next()?.parse().ok()?, it.next()?.parse().ok()?);
    if it.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) || !(2000..=2100).contains(&y) {
        return None;
    }
    Some(days_from_civil(y, m, d))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum Yield {
    High,
    Medium,
    Low,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct Priority {
    pub principle: PrincipleRef,
    pub skill_name: String,
    pub yield_level: Yield,
    pub mastery: Option<f64>,
    pub level: Level,
    /// Where to go study it.
    pub study: Option<crate::study::StudyView>,
}

#[derive(Debug, Clone, Copy, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    /// Short practice test to find a starting point.
    Diagnostic,
    /// A set on today's top-priority principles.
    PowerSet,
    /// Missed questions due for spaced review.
    Review,
    /// Full-length timed practice test.
    FullTest,
    /// Read today's memory hacks.
    Hacks,
    /// Read the test-day tactics.
    Tactics,
    /// Continue the guided journey.
    Journey,
    /// Pick a test date to get a plan.
    SetDate,
    Rest,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct PlanTask {
    pub kind: TaskKind,
    pub title: String,
    pub detail: String,
    /// For a power set: the principles to practice.
    pub principles: Vec<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct Plan {
    /// YYYY-MM-DD, if set.
    pub test_date: Option<String>,
    #[ts(type = "number | null")]
    pub days_left: Option<i64>,
    /// How many principles to aim for before the test.
    pub focus_count: u32,
    pub priorities: Vec<Priority>,
    pub today: Vec<PlanTask>,
    pub hacks: Vec<HackItem>,
}

fn yield_of(state: &AppState, principle: &str) -> Yield {
    match state.bank.strategy(state.serve_drafts) {
        Some(s) if s.high_yield.iter().any(|p| p == principle) => Yield::High,
        Some(s) if s.medium_yield.iter().any(|p| p == principle) => Yield::Medium,
        _ => Yield::Low,
    }
}

fn weight(y: Yield) -> f64 {
    match y {
        Yield::High => 3.0,
        Yield::Medium => 2.0,
        Yield::Low => 1.0,
    }
}

pub async fn get_plan(
    State(state): State<Arc<AppState>>,
    user: CurrentUser,
    Query(tz): Query<TzParams>,
    Query(sp): Query<crate::api::SubjectParam>,
) -> Result<Json<Plan>, ApiError> {
    let subject = sp.resolve(&state)?.to_string();
    let test_date: Option<String> = sqlx::query_scalar("SELECT test_date FROM users WHERE id = ?")
        .bind(user.id)
        .fetch_one(&state.pool)
        .await?;
    let today = (now_unix() + tz.offset() * 60).div_euclid(86400);
    let days_left = test_date.as_deref().and_then(parse_date).map(|d| d - today);

    let stats = LearnerStats::load(&state, user.id).await?;
    let flagged = crate::flags::flagged_ids(&state.pool).await?;
    let mut scored: Vec<(f64, usize, Priority)> = state
        .bank
        .principles
        .iter()
        .enumerate()
        .filter(|(_, p)| state.bank.skill_subject(&p.skill) == subject)
        .filter(|(_, p)| !servable(&state, &flagged, principle_question_ids(&state, &p.id)).is_empty())
        .map(|(i, p)| {
            let (_, _, mastery, level) = stats.principle(&p.id);
            let y = yield_of(&state, &p.id);
            let need = 1.0 - mastery.unwrap_or(0.4);
            let pr = Priority {
                principle: principle_ref(&state, &p.id),
                skill_name: state.bank.skill(&p.skill).map(|s| s.name.clone()).unwrap_or_default(),
                yield_level: y,
                mastery,
                level,
                study: crate::study::study_view(&state, &p.id),
            };
            (weight(y) * need, i, pr)
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal).then(a.1.cmp(&b.1)));
    let focus_count = match days_left {
        Some(d) if d >= 0 => (d * 4 + 4).clamp(8, 40) as u32,
        _ => 15,
    };
    let priorities: Vec<Priority> = scored.into_iter().map(|(_, _, p)| p).take(focus_count as usize).collect();

    let tests_done: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tests WHERE user_id = ? AND finished_at IS NOT NULL")
        .bind(user.id)
        .fetch_one(&state.pool)
        .await?;
    let recent_full: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tests WHERE user_id = ? AND kind = 'full' AND finished_at >= ?",
    )
    .bind(user.id)
    .bind(now_unix() - 3 * 86400)
    .fetch_one(&state.pool)
    .await?;
    let review_due = crate::spaced::due_ids(&state.pool, user.id).await?.len();

    let power: Vec<String> = priorities
        .iter()
        .filter(|p| p.level != Level::Strong)
        .take(3)
        .map(|p| p.principle.id.clone())
        .collect();
    let power_names: Vec<String> = priorities
        .iter()
        .filter(|p| power.contains(&p.principle.id))
        .map(|p| p.principle.name.clone())
        .collect();

    let mut today_tasks = Vec::new();
    let mut task = |kind, title: &str, detail: String, principles: Vec<String>| {
        today_tasks.push(PlanTask { kind, title: title.into(), detail, principles })
    };
    match days_left {
        Some(d) if d < 0 => {
            task(TaskKind::SetDate, "Set your next test date", "Your saved date has passed.".into(), vec![]);
        }
        Some(0) => {
            task(TaskKind::Tactics, "Read the test-day tactics", "Five minutes. Then put the books away.".into(), vec![]);
            task(TaskKind::Rest, "You're ready — go do it", "Eat, bring your ID, arrive early.".into(), vec![]);
        }
        Some(1) => {
            task(TaskKind::Hacks, "Review your hacks", "Quick read of the memory tricks below.".into(), vec![]);
            if review_due > 0 {
                task(TaskKind::Review, "Light review", format!("{review_due} missed questions, nothing new."), vec![]);
            }
            task(TaskKind::Tactics, "Read the test-day tactics", "Then rest. Sleep beats cramming.".into(), vec![]);
        }
        _ => {
            if tests_done == 0 {
                task(TaskKind::Diagnostic, "Take a 10-question check-up", "Shows where to spend your time.".into(), vec![]);
            }
            if !power.is_empty() {
                task(
                    TaskKind::PowerSet,
                    "Power set",
                    format!("10 questions on: {}", power_names.join(", ")),
                    power.clone(),
                );
            }
            if review_due > 0 {
                task(TaskKind::Review, "Review missed questions", format!("{review_due} due now."), vec![]);
            }
            if matches!(days_left, Some(2..=3)) && recent_full == 0 {
                task(TaskKind::FullTest, "Full-length practice test", "Timed, like the real thing.".into(), vec![]);
            }
            task(TaskKind::Hacks, "Learn today's hacks", "Memory tricks for your power set.".into(), vec![]);
            if days_left.is_none() {
                task(TaskKind::Journey, "Continue your journey", "Next stage, a few minutes.".into(), vec![]);
                task(TaskKind::SetDate, "Set your test date", "Get a day-by-day plan.".into(), vec![]);
            }
        }
    }

    let hack_ids: Vec<String> = if matches!(days_left, Some(0..=1)) {
        priorities.iter().take(8).map(|p| p.principle.id.clone()).collect()
    } else {
        power.clone()
    };
    let hacks = hack_ids.iter().filter_map(|p| hack_item(&state, p)).collect();
    Ok(Json(Plan { test_date, days_left, focus_count, priorities, today: today_tasks, hacks }))
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct SetTestDate {
    /// YYYY-MM-DD, or null to clear.
    pub test_date: Option<String>,
}

pub async fn set_test_date(
    State(state): State<Arc<AppState>>,
    user: CurrentUser,
    Json(req): Json<SetTestDate>,
) -> Result<StatusCode, ApiError> {
    let date = match req.test_date.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => None,
        Some(s) => {
            parse_date(s).ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "Use a date like 2026-12-01."))?;
            Some(s.to_string())
        }
    };
    sqlx::query("UPDATE users SET test_date = ? WHERE id = ?")
        .bind(date)
        .bind(user.id)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct StrategyView {
    pub tactics: Vec<ged_content::Tactic>,
    pub formulas: Vec<ged_content::Formula>,
    /// Every principle's hack, in skill order.
    pub hacks: Vec<HackItem>,
}

/// Test-day tactics (shared), and the formula sheet and memory hacks for a
/// subject (`?subject=`, Math by default).
pub async fn get_strategy(
    State(state): State<Arc<AppState>>,
    _user: CurrentUser,
    Query(sp): Query<crate::api::SubjectParam>,
) -> Result<Json<StrategyView>, ApiError> {
    let subject = sp.resolve(&state)?.to_string();
    let (tactics, formulas) = match state.bank.strategy(state.serve_drafts) {
        Some(s) => (s.tactic.clone(), s.formula.clone()),
        None => (Vec::new(), Vec::new()),
    };
    // The formula sheet is Math's.
    let formulas = if subject == "math" { formulas } else { Vec::new() };
    let hacks = state
        .bank
        .principles
        .iter()
        .filter(|p| state.bank.skill_subject(&p.skill) == subject)
        .filter_map(|p| hack_item(&state, &p.id))
        .collect();
    Ok(Json(StrategyView { tactics, formulas, hacks }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2026, 11, 11) - days_from_civil(2026, 11, 4), 7);
        assert_eq!(parse_date("2026-12-01"), Some(days_from_civil(2026, 12, 1)));
        assert_eq!(parse_date("2026-13-01"), None);
        assert_eq!(parse_date("next week"), None);
    }
}
