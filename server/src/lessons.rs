//! Full lessons: one idea each, shown one step at a time
//! (`content/lessons/`, `docs/LESSON_STYLE.md`).

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Serialize;
use ts_rs::TS;

use crate::AppState;
use crate::api::{ApiError, PrincipleRef, principle_ref};
use crate::auth::CurrentUser;

/// A lesson in a list.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct LessonRef {
    pub id: String,
    pub title: String,
    pub draft: bool,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct LessonStepView {
    /// One line of work; math in `$...$`.
    pub work: String,
    pub why: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct WorkedExampleView {
    pub title: String,
    pub steps: Vec<LessonStepView>,
    pub answer: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct TermView {
    pub term: String,
    pub means: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct MistakeView {
    pub wrong: String,
    pub right: String,
    pub why: String,
}

/// A question inside a lesson. The answer and solution travel with it so
/// feedback is instant; these are practice and are not scored.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct TryItView {
    pub prompt: String,
    /// Empty for a typed answer.
    pub choices: Vec<String>,
    pub answer: String,
    pub solution: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct LessonView {
    pub id: String,
    pub principle: PrincipleRef,
    pub skill_name: String,
    pub title: String,
    pub why: String,
    pub idea: String,
    pub examples: Vec<WorkedExampleView>,
    pub rule: String,
    pub terms: Vec<TermView>,
    pub mistake: MistakeView,
    pub tries: Vec<TryItView>,
    pub recap: Vec<String>,
    /// True if the lesson has not been reviewed yet (dev mode only).
    pub draft: bool,
    /// Whether the principle has a Quick review card.
    pub has_quick_review: bool,
}

/// The lessons for a principle that learners may see, in file order.
pub(crate) fn lesson_refs(state: &AppState, principle: &str) -> Vec<LessonRef> {
    state
        .bank
        .lessons
        .iter()
        .filter(|l| l.principle == principle && crate::content::Bank::unit_servable(l, state.serve_drafts))
        .map(|l| LessonRef { id: l.id.clone(), title: l.title.clone(), draft: l.status == ged_content::Status::Draft })
        .collect()
}

pub async fn get_lesson(
    State(state): State<Arc<AppState>>,
    _user: CurrentUser,
    Path(id): Path<String>,
) -> Result<Json<LessonView>, ApiError> {
    let l = state
        .bank
        .lessons
        .iter()
        .find(|l| l.id == id && crate::content::Bank::unit_servable(l, state.serve_drafts))
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "No such lesson."))?;
    let skill_name = state
        .bank
        .principle(&l.principle)
        .and_then(|p| state.bank.skill(&p.skill))
        .map(|s| s.name.clone())
        .unwrap_or_default();
    let t = |s: &str| s.trim().to_string();
    Ok(Json(LessonView {
        id: l.id.clone(),
        principle: principle_ref(&state, &l.principle),
        skill_name,
        title: t(&l.title),
        why: t(&l.why),
        idea: t(&l.idea),
        examples: l
            .example
            .iter()
            .map(|e| WorkedExampleView {
                title: t(&e.title),
                steps: e.steps.iter().map(|s| LessonStepView { work: t(&s.work), why: t(&s.why) }).collect(),
                answer: t(&e.answer),
            })
            .collect(),
        rule: t(&l.rule.text),
        terms: l.rule.terms.iter().map(|x| TermView { term: t(&x.term), means: t(&x.means) }).collect(),
        mistake: MistakeView { wrong: t(&l.mistake.wrong), right: t(&l.mistake.right), why: t(&l.mistake.why) },
        tries: l
            .tries
            .iter()
            .map(|x| TryItView { prompt: t(&x.prompt), choices: x.choices.clone(), answer: t(&x.answer), solution: t(&x.solution) })
            .collect(),
        recap: l.recap.iter().map(|r| t(r)).collect(),
        draft: l.status == ged_content::Status::Draft,
        has_quick_review: state.bank.card(&l.principle, state.serve_drafts).is_some(),
    }))
}
