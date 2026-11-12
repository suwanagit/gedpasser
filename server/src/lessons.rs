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
use crate::figures::FigureView;

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
    pub say: String,
    /// Lines of written work; `**bold**` marks what to notice.
    pub work: Vec<String>,
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
pub struct WorkedExampleView {
    pub title: String,
    pub steps: Vec<LessonStepView>,
    pub answer: String,
    pub check: String,
    pub turn: TryItView,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct MistakeView {
    pub wrong: String,
    pub right: String,
    pub why: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct LessonView {
    pub id: String,
    pub principle: PrincipleRef,
    pub skill_name: String,
    pub title: String,
    pub goal: String,
    pub problem: String,
    /// Said after example 1, which solves the problem.
    pub problem_answer: String,
    pub idea: String,
    pub figures: Vec<FigureView>,
    pub examples: Vec<WorkedExampleView>,
    pub how_to: Vec<String>,
    pub hook: String,
    pub mistakes: Vec<MistakeView>,
    pub practice: Vec<TryItView>,
    /// The small worked example on the lesson's Quick review.
    pub review_example: String,
    /// True if the lesson has not been reviewed yet (dev mode only).
    pub draft: bool,
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
    let q = |x: &ged_content::TryIt| TryItView {
        prompt: t(&x.prompt),
        choices: x.choices.clone(),
        answer: t(&x.answer),
        solution: t(&x.solution),
    };
    Ok(Json(LessonView {
        id: l.id.clone(),
        principle: principle_ref(&state, &l.principle),
        skill_name,
        title: t(&l.title),
        goal: t(&l.goal),
        problem: t(&l.problem.text),
        problem_answer: t(&l.problem.answer),
        idea: t(&l.idea.text),
        figures: l.idea.figures.iter().map(FigureView::new).collect(),
        examples: l
            .example
            .iter()
            .map(|e| WorkedExampleView {
                title: t(&e.title),
                steps: e.steps.iter().map(|s| LessonStepView { say: t(&s.say), work: s.work.iter().map(|w| t(w)).collect() }).collect(),
                answer: t(&e.answer),
                check: t(&e.check),
                turn: q(&e.turn),
            })
            .collect(),
        how_to: l.how_to.iter().map(|h| t(h)).collect(),
        hook: t(&l.hook),
        mistakes: l.mistake.iter().map(|m| MistakeView { wrong: t(&m.wrong), right: t(&m.right), why: t(&m.why) }).collect(),
        practice: l.practice.iter().map(q).collect(),
        review_example: t(&l.review.example),
        draft: l.status == ged_content::Status::Draft,
    }))
}
