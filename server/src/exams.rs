//! Practice tests: short, long, or about the real test's length. Timed, no
//! feedback until the end, then a report by skill.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::AppState;
use crate::api::{ApiError, PracticeQuestion, PrincipleRef, SubjectParam, practice_question, principle_ref, random_u32, servable};
use crate::auth::CurrentUser;
use crate::content::grade;
use crate::spaced::now_unix;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum TestKind {
    Short,
    Long,
    Full,
}

impl TestKind {
    fn as_str(self) -> &'static str {
        match self {
            TestKind::Short => "short",
            TestKind::Long => "long",
            TestKind::Full => "full",
        }
    }

    /// (questions, time limit in seconds) for a subject, from subjects.toml.
    pub fn shape(self, subject: &ged_content::Subject) -> (usize, i64) {
        subject
            .test
            .iter()
            .find(|t| t.kind == self.as_str())
            .map(|t| (t.questions as usize, t.minutes as i64 * 60))
            .unwrap_or((10, 25 * 60))
    }
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct NewTest {
    pub kind: TestKind,
    /// Which subject; Math when left out.
    #[serde(default)]
    pub subject: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct TestPaper {
    #[ts(type = "number")]
    pub id: i64,
    pub kind: TestKind,
    pub subject: String,
    pub time_limit_secs: u32,
    pub questions: Vec<PracticeQuestion>,
}

fn shuffle<T>(v: &mut [T]) {
    for i in (1..v.len()).rev() {
        v.swap(i, random_u32() as usize % (i + 1));
    }
}

/// A question, or a set of questions sharing one stimulus; tests keep a set
/// together, as the real test does.
struct Unit {
    ids: Vec<String>,
    skill: String,
    domain: String,
}

/// Pick about `n` questions for a subject: each domain gets its share of the
/// real test (for Math, 45% quantitative), spread round-robin across
/// skills, random within each, with stimulus sets kept together.
pub(crate) fn pick(state: &AppState, subject: &ged_content::Subject, n: usize, flagged: &HashSet<String>) -> Vec<String> {
    // Build units from every servable question in the subject.
    let mut units: Vec<Unit> = Vec::new();
    let mut by_stimulus: HashMap<String, usize> = HashMap::new();
    let mut ids: Vec<&String> = state
        .bank
        .skills
        .iter()
        .filter(|s| s.subject == subject.id)
        .flat_map(|s| state.bank.by_skill.get(&s.id).map(Vec::as_slice).unwrap_or(&[]))
        .collect();
    ids.sort();
    for id in servable(state, flagged, ids.into_iter()) {
        let q = &state.bank.questions[id];
        match &q.stimulus {
            Some(st) if by_stimulus.contains_key(st) => units[by_stimulus[st]].ids.push(id.clone()),
            other => {
                if let Some(st) = other {
                    by_stimulus.insert(st.clone(), units.len());
                }
                units.push(Unit { ids: vec![id.clone()], skill: q.skill.clone(), domain: state.bank.question_domain(q) });
            }
        }
    }
    // A passage can carry more questions for practice, but a test shows at
    // most `set_max` of them (default 3, as on the real Science and Social
    // Studies tests), picked at random and kept in file order.
    let set_max = subject.set_max.unwrap_or(3).max(1);
    for u in &mut units {
        if u.ids.len() > set_max {
            let mut keep: Vec<usize> = (0..u.ids.len()).collect();
            shuffle(&mut keep);
            keep.truncate(set_max);
            keep.sort();
            u.ids = keep.into_iter().map(|i| u.ids[i].clone()).collect();
        }
    }
    let mut out: Vec<Vec<String>> = Vec::new();
    let mut given = 0;
    let last = subject.domain.len().saturating_sub(1);
    for (i, d) in subject.domain.iter().enumerate() {
        let want = if i == last { n.saturating_sub(given) } else { ((n as f64) * d.share).round() as usize };
        // One shuffled pool per skill in this domain.
        let mut skills: Vec<&str> = units.iter().filter(|u| u.domain == d.id).map(|u| u.skill.as_str()).collect();
        skills.sort();
        skills.dedup();
        let mut pools: Vec<Vec<&Unit>> = skills
            .iter()
            .map(|sk| {
                let mut v: Vec<&Unit> = units.iter().filter(|u| u.domain == d.id && u.skill == *sk).collect();
                shuffle(&mut v);
                v
            })
            .collect();
        shuffle(&mut pools);
        let mut taken = 0;
        let mut round = 0;
        while taken < want && pools.iter().any(|p| p.len() > round) {
            for p in &pools {
                if taken < want && p.len() > round {
                    let u = p[round];
                    // A set bigger than what's left is skipped, unless nothing else fits.
                    if taken + u.ids.len() <= want || taken == 0 {
                        out.push(u.ids.clone());
                        taken += u.ids.len();
                    }
                }
            }
            round += 1;
        }
        given += taken;
    }
    shuffle(&mut out);
    out.into_iter().flatten().collect()
}

pub async fn new_test(
    State(state): State<Arc<AppState>>,
    user: CurrentUser,
    Json(req): Json<NewTest>,
) -> Result<Json<TestPaper>, ApiError> {
    let subject_id = req.subject.clone().unwrap_or_else(|| "math".into());
    let subject = state
        .bank
        .subject(&subject_id)
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "No such subject."))?;
    let (n, limit) = req.kind.shape(subject);
    let flagged = crate::flags::flagged_ids(&state.pool).await?;
    let ids = pick(&state, subject, n, &flagged);
    if ids.is_empty() {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "No reviewed questions are available yet."));
    }
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO tests (user_id, kind, subject, question_ids, time_limit_secs, started_at) VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(user.id)
    .bind(req.kind.as_str())
    .bind(&subject_id)
    .bind(serde_json::to_string(&ids).unwrap_or_default())
    .bind(limit)
    .bind(now_unix())
    .fetch_one(&state.pool)
    .await?;
    let questions = ids
        .iter()
        .map(|q| practice_question(&state, &state.bank.questions[q]))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(TestPaper { id, kind: req.kind, subject: subject_id, time_limit_secs: limit as u32, questions }))
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct TestAnswer {
    pub question_id: String,
    /// Empty if skipped.
    pub response: String,
    pub time_ms: u32,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct TestSubmission {
    pub answers: Vec<TestAnswer>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct SkillScore {
    pub skill: String,
    pub name: String,
    pub correct: u32,
    pub total: u32,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ItemResult {
    pub question_id: String,
    pub response: String,
    pub correct: bool,
    pub answer: String,
    pub explanation: String,
    pub common_mistake: String,
    pub principle: PrincipleRef,
    pub hack: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct TestReport {
    #[ts(type = "number")]
    pub id: i64,
    pub kind: TestKind,
    pub correct: u32,
    pub total: u32,
    pub percent: u32,
    /// "ready" (80%+), "close" (65–79), "building" (50–64), "early" (<50).
    /// Our practice estimate, not an official GED score.
    pub band: String,
    pub time_used_secs: u32,
    pub by_skill: Vec<SkillScore>,
    pub items: Vec<ItemResult>,
}

pub fn band(percent: u32) -> &'static str {
    match percent {
        80.. => "ready",
        65..=79 => "close",
        50..=64 => "building",
        _ => "early",
    }
}

pub async fn submit_test(
    State(state): State<Arc<AppState>>,
    user: CurrentUser,
    Path(id): Path<i64>,
    Json(sub): Json<TestSubmission>,
) -> Result<Json<TestReport>, ApiError> {
    let row: Option<(String, String, i64, Option<i64>)> =
        sqlx::query_as("SELECT kind, question_ids, started_at, finished_at FROM tests WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user.id)
            .fetch_optional(&state.pool)
            .await?;
    let (kind, ids_json, started_at, finished) = row.ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "No such test."))?;
    if finished.is_some() {
        return Err(ApiError::new(StatusCode::CONFLICT, "This test was already submitted."));
    }
    let kind = match kind.as_str() {
        "short" => TestKind::Short,
        "long" => TestKind::Long,
        _ => TestKind::Full,
    };
    let ids: Vec<String> = serde_json::from_str(&ids_json).unwrap_or_default();
    let given: HashMap<&str, &TestAnswer> = sub.answers.iter().map(|a| (a.question_id.as_str(), a)).collect();
    let mut items = Vec::new();
    let mut per_skill: HashMap<String, (u32, u32)> = HashMap::new();
    let mut correct_n = 0;
    for qid in &ids {
        let Some(q) = state.bank.questions.get(qid) else { continue };
        let a = given.get(qid.as_str());
        let response: String = a.map(|a| a.response.trim().chars().take(200).collect()).unwrap_or_default();
        let correct = !response.is_empty() && grade(q, &response);
        if !response.is_empty() {
            sqlx::query(
                "INSERT INTO attempts (user_id, question_id, principle_id, response, correct, time_ms, mode, content_hash, hint_used)
                 VALUES (?, ?, ?, ?, ?, ?, 'test', ?, 0)",
            )
            .bind(user.id)
            .bind(&q.id)
            .bind(&q.principle)
            .bind(&response)
            .bind(correct as i64)
            .bind(a.map(|a| a.time_ms.min(3_600_000)).unwrap_or(0) as i64)
            .bind(&state.bank.hashes[&q.id])
            .execute(&state.pool)
            .await?;
            crate::spaced::record(&state.pool, user.id, &q.id, correct).await?;
        }
        if correct {
            correct_n += 1;
        }
        let e = per_skill.entry(q.skill.clone()).or_default();
        e.1 += 1;
        if correct {
            e.0 += 1;
        }
        items.push(ItemResult {
            question_id: q.id.clone(),
            response,
            correct,
            answer: q.answer_text(),
            explanation: q.explanation.trim().to_string(),
            common_mistake: q.common_mistake.trim().to_string(),
            principle: principle_ref(&state, &q.principle),
            hack: state.bank.card(&q.principle, state.serve_drafts).map(|c| c.hack.trim().to_string()),
        });
    }
    let total = items.len() as u32;
    let now = now_unix();
    sqlx::query("UPDATE tests SET finished_at = ?, correct = ?, total = ? WHERE id = ?")
        .bind(now)
        .bind(correct_n as i64)
        .bind(total as i64)
        .bind(id)
        .execute(&state.pool)
        .await?;
    let percent = if total == 0 { 0 } else { (correct_n * 100 + total / 2) / total };
    let by_skill = state
        .bank
        .skills
        .iter()
        .filter_map(|s| {
            per_skill.get(&s.id).map(|(c, t)| SkillScore { skill: s.id.clone(), name: s.name.clone(), correct: *c, total: *t })
        })
        .collect();
    Ok(Json(TestReport {
        id,
        kind,
        correct: correct_n,
        total,
        percent,
        band: band(percent).to_string(),
        time_used_secs: (now - started_at).max(0) as u32,
        by_skill,
        items,
    }))
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct TestSummary {
    #[ts(type = "number")]
    pub id: i64,
    pub kind: TestKind,
    pub correct: u32,
    pub total: u32,
    pub percent: u32,
    #[ts(type = "number")]
    pub finished_at: i64,
}

pub async fn list_tests(
    State(state): State<Arc<AppState>>,
    user: CurrentUser,
    Query(sp): Query<SubjectParam>,
) -> Result<Json<Vec<TestSummary>>, ApiError> {
    let subject = sp.resolve(&state)?;
    let rows: Vec<(i64, String, i64, i64, i64)> = sqlx::query_as(
        "SELECT id, kind, correct, total, finished_at FROM tests
         WHERE user_id = ? AND subject = ? AND finished_at IS NOT NULL ORDER BY finished_at DESC LIMIT 20",
    )
    .bind(user.id)
    .bind(subject)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|(id, kind, c, t, f)| TestSummary {
                id,
                kind: match kind.as_str() {
                    "short" => TestKind::Short,
                    "long" => TestKind::Long,
                    _ => TestKind::Full,
                },
                correct: c as u32,
                total: t as u32,
                percent: if t == 0 { 0 } else { ((c * 100 + t / 2) / t) as u32 },
                finished_at: f,
            })
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::band;

    #[test]
    fn bands() {
        assert_eq!(band(100), "ready");
        assert_eq!(band(80), "ready");
        assert_eq!(band(79), "close");
        assert_eq!(band(50), "building");
        assert_eq!(band(10), "early");
    }
}
