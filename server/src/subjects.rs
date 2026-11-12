//! The GED subjects the app covers, and whether each one is ready to use.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use serde::Serialize;
use ts_rs::TS;

use crate::AppState;
use crate::api::{ApiError, servable};
use crate::auth::CurrentUser;

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct TestSize {
    /// "short" | "long" | "full".
    pub kind: String,
    pub questions: u32,
    pub minutes: u32,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct SubjectView {
    pub id: String,
    pub name: String,
    /// True once the subject has questions learners may be served.
    pub ready: bool,
    pub calculator: bool,
    pub tests: Vec<TestSize>,
    /// Week path: sets passed and sets in all.
    pub stages_passed: u32,
    pub stages_total: u32,
}

pub async fn list_subjects(State(state): State<Arc<AppState>>, user: CurrentUser) -> Result<Json<Vec<SubjectView>>, ApiError> {
    let flagged = crate::flags::flagged_ids(&state.pool).await?;
    let mut out = Vec::new();
    for s in &state.bank.subjects {
        let ids = state
            .bank
            .skills
            .iter()
            .filter(|k| k.subject == s.id)
            .flat_map(|k| state.bank.by_skill.get(&k.id).map(Vec::as_slice).unwrap_or(&[]));
        let ready = !servable(&state, &flagged, ids).is_empty();
        let path = crate::journey::build_view(&state, user.id, &s.id).await?;
        out.push(SubjectView {
            id: s.id.clone(),
            name: s.name.clone(),
            ready,
            calculator: s.calculator,
            tests: s.test.iter().map(|t| TestSize { kind: t.kind.clone(), questions: t.questions, minutes: t.minutes }).collect(),
            stages_passed: path.passed,
            stages_total: path.stages.len() as u32,
        });
    }
    Ok(Json(out))
}
