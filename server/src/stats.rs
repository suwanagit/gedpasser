//! XP, level and daily streak — all derived from stored activity, never
//! stored themselves.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::AppState;
use crate::api::{ApiError, LearnerStats};
use crate::auth::CurrentUser;
use crate::mastery::Level;

/// XP per answer, and per correct answer on top.
pub const XP_ANSWER: u32 = 2;
pub const XP_CORRECT: u32 = 8;
/// Journey stage passed, plus per star.
pub const XP_STAGE: u32 = 25;
pub const XP_STAR: u32 = 10;
pub const XP_TEST: u32 = 50;
pub const LEVEL_SIZE: u32 = 250;

pub fn answer_xp(correct: bool) -> u32 {
    XP_ANSWER + if correct { XP_CORRECT } else { 0 }
}

pub fn stage_xp(stars: u32) -> u32 {
    if stars == 0 { 0 } else { XP_STAGE + XP_STAR * stars }
}

#[derive(Debug, Deserialize)]
pub struct TzParams {
    /// Minutes east of UTC (Alaska is -540, or -480 in summer). Default 0.
    pub tz: Option<i64>,
}

impl TzParams {
    pub fn offset(&self) -> i64 {
        self.tz.unwrap_or(0).clamp(-14 * 60, 14 * 60)
    }
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct Stats {
    pub xp: u32,
    pub level: u32,
    /// XP earned inside the current level, out of `level_size`.
    pub level_xp: u32,
    pub level_size: u32,
    /// Days in a row with at least one answer, ending today (or yesterday).
    pub streak_days: u32,
    pub answered_today: u32,
    pub review_due: u32,
    /// Skills at "strong" mastery — a badge each.
    pub badges: Vec<String>,
}

pub async fn total_xp(state: &AppState, user_id: i64) -> Result<u32, ApiError> {
    let (answers, correct): (i64, i64) =
        sqlx::query_as("SELECT COUNT(*), COALESCE(SUM(correct), 0) FROM attempts WHERE user_id = ?")
            .bind(user_id)
            .fetch_one(&state.pool)
            .await?;
    let stars: Vec<i64> = sqlx::query_scalar("SELECT stars FROM journey_progress WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(&state.pool)
        .await?;
    let tests: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tests WHERE user_id = ? AND finished_at IS NOT NULL")
        .bind(user_id)
        .fetch_one(&state.pool)
        .await?;
    let xp = answers as u32 * XP_ANSWER
        + correct as u32 * XP_CORRECT
        + stars.iter().map(|&s| stage_xp(s as u32)).sum::<u32>()
        + tests as u32 * XP_TEST;
    Ok(xp)
}

/// Consecutive days ending today or yesterday, from distinct local dates
/// (as day numbers) sorted newest first.
pub fn streak(days_desc: &[i64], today: i64) -> u32 {
    let Some(&first) = days_desc.first() else { return 0 };
    if first < today - 1 {
        return 0;
    }
    let mut n = 1;
    for w in days_desc.windows(2) {
        if w[0] - w[1] == 1 {
            n += 1;
        } else {
            break;
        }
    }
    n
}

pub async fn get_stats(
    State(state): State<Arc<AppState>>,
    user: CurrentUser,
    Query(tz): Query<TzParams>,
) -> Result<Json<Stats>, ApiError> {
    let xp = total_xp(&state, user.id).await?;
    let offset = tz.offset();
    // Local day numbers (days since 1970-01-01) of every answer.
    let days: Vec<i64> = sqlx::query_scalar(
        "SELECT DISTINCT CAST((CAST(strftime('%s', answered_at) AS INTEGER) + ? * 60) / 86400 AS INTEGER) AS d
         FROM attempts WHERE user_id = ? ORDER BY d DESC",
    )
    .bind(offset)
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;
    let today = (crate::spaced::now_unix() + offset * 60).div_euclid(86400);
    let answered_today: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM attempts WHERE user_id = ?
           AND CAST((CAST(strftime('%s', answered_at) AS INTEGER) + ? * 60) / 86400 AS INTEGER) = ?",
    )
    .bind(user.id)
    .bind(offset)
    .bind(today)
    .fetch_one(&state.pool)
    .await?;
    let review_due = crate::spaced::due_ids(&state.pool, user.id).await?.len() as u32;
    let learner = LearnerStats::load(&state, user.id).await?;
    let badges = state
        .bank
        .skills
        .iter()
        .filter(|s| learner.skill(&s.id).3 == Level::Strong)
        .map(|s| s.id.clone())
        .collect();
    Ok(Json(Stats {
        xp,
        level: xp / LEVEL_SIZE + 1,
        level_xp: xp % LEVEL_SIZE,
        level_size: LEVEL_SIZE,
        streak_days: streak(&days, today),
        answered_today: answered_today as u32,
        review_due,
        badges,
    }))
}

#[cfg(test)]
mod tests {
    use super::streak;

    #[test]
    fn streaks() {
        assert_eq!(streak(&[], 100), 0);
        assert_eq!(streak(&[100, 99, 98, 96], 100), 3);
        assert_eq!(streak(&[99, 98], 100), 2, "yesterday still counts");
        assert_eq!(streak(&[97, 96], 100), 0, "broken");
    }
}
