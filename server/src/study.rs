//! Where to go study a principle on your own (`content/study.toml`).

use serde::Serialize;
use ts_rs::TS;

use crate::AppState;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct StudyLinkView {
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct StudyView {
    /// A few words to search for; the app builds a web-search link from
    /// these words only.
    pub search: String,
    pub links: Vec<StudyLinkView>,
}

/// The study topic for a principle, if learners may see it.
pub(crate) fn study_view(state: &AppState, principle: &str) -> Option<StudyView> {
    let file = state.bank.study(state.serve_drafts)?;
    let t = file.topic.iter().find(|t| t.principle == principle)?;
    Some(StudyView {
        search: t.search.trim().to_string(),
        links: t.links.iter().map(|l| StudyLinkView { title: l.title.trim().to_string(), url: l.url.trim().to_string() }).collect(),
    })
}
