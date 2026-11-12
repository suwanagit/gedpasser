//! Subjects, shared stimuli and the question formats beyond multiple
//! choice and numeric fill-in.

use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use crate::extra::figure_problems;
use crate::schema::{Question, QuestionType, Skill, Stimulus, StimulusFile, Subject, SubjectFile};
use crate::{Level, Report, read_toml};

pub const TEST_KINDS: [&str; 3] = ["short", "long", "full"];

fn is_plain_id(s: &str) -> bool {
    !s.is_empty()
        && s.starts_with(|c: char| c.is_ascii_lowercase())
        && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Load `root/subjects.toml`. A bank without one has just Math.
pub(crate) fn check_subjects(root: &Path, report: &mut Report) {
    let path = root.join("subjects.toml");
    if !path.exists() {
        report.subjects = vec![Subject::default_math()];
        return;
    }
    let Some(file) = read_toml::<SubjectFile>(&path, report) else {
        report.subjects = vec![Subject::default_math()];
        return;
    };
    let mut seen = HashSet::new();
    let mut errs: Vec<(String, String)> = Vec::new();
    for s in &file.subject {
        let mut err = |m: String| errs.push((s.id.clone(), m));
        if !is_plain_id(&s.id) {
            err("subject id must be lowercase letters, digits and hyphens".into());
        }
        if !seen.insert(s.id.clone()) {
            err("duplicate subject id".into());
        }
        if s.name.trim().is_empty() {
            err("name must not be empty".into());
        }
        if s.domain.is_empty() {
            err("a subject needs at least one domain".into());
        }
        let mut domains = HashSet::new();
        for d in &s.domain {
            if d.id.trim().is_empty() || d.name.trim().is_empty() {
                err("every domain needs an id and a name".into());
            }
            if !domains.insert(d.id.as_str()) {
                err(format!("duplicate domain {:?}", d.id));
            }
            if !(d.share > 0.0 && d.share <= 1.0) {
                err(format!("domain {:?} share must be between 0 and 1", d.id));
            }
        }
        let total: f64 = s.domain.iter().map(|d| d.share).sum();
        if !s.domain.is_empty() && (total - 1.0).abs() > 1e-6 {
            err(format!("domain shares add up to {total}, not 1"));
        }
        let kinds: Vec<&str> = s.test.iter().map(|t| t.kind.as_str()).collect();
        if kinds != TEST_KINDS {
            err("tests must be exactly short, long and full, in that order".into());
        }
        for t in &s.test {
            if t.questions == 0 || t.minutes == 0 {
                err(format!("test {:?} needs at least one question and one minute", t.kind));
            }
        }
    }
    if !file.subject.iter().any(|s| s.id == "math") {
        errs.push(("math".into(), "subjects.toml must list the math subject".into()));
    }
    for (id, m) in errs {
        report.push(Level::Error, &path, Some(&id), m);
    }
    report.subjects = file.subject;
}

/// Load every `root/stimuli/<subject>.toml`.
pub(crate) fn load_stimuli(root: &Path, report: &mut Report) {
    let dir = root.join("stimuli");
    let mut paths: Vec<PathBuf> = match std::fs::read_dir(&dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect(),
        Err(_) => Vec::new(), // no stimuli yet
    };
    paths.sort();
    let mut loaded: Vec<(PathBuf, Stimulus)> = Vec::new();
    let mut diags: Vec<(PathBuf, Option<String>, String)> = Vec::new();
    let mut seen = HashSet::new();
    for path in paths {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
        if !report.subjects.iter().any(|s| s.id == stem) {
            diags.push((path.clone(), None, format!("file name {stem:?} is not a subject id in subjects.toml")));
        }
        let mut sub = Report::default();
        if let Some(file) = read_toml::<StimulusFile>(&path, &mut sub) {
            for s in file.stimulus {
                let mut err = |m: String| diags.push((path.clone(), Some(s.id.clone()), m));
                if !s.id.starts_with(&format!("{stem}-")) || !is_plain_id(&s.id) {
                    err(format!("stimulus id must look like \"{stem}-name\""));
                }
                if !seen.insert(s.id.clone()) {
                    err("duplicate stimulus id".into());
                }
                if s.text.trim().is_empty() && s.table.is_none() && s.figure.is_none() {
                    err("a stimulus needs text, a table or a figure".into());
                }
                if s.source.trim().is_empty() {
                    err("source must not be empty (use \"Original\" for our own writing)".into());
                }
                if let Some(t) = &s.table {
                    if t.headers.is_empty() || t.rows.is_empty() {
                        err("a table needs headers and at least one row".into());
                    }
                    if t.rows.iter().any(|r| r.len() != t.headers.len()) {
                        err("every table row must have one cell per header".into());
                    }
                }
                if let Some(f) = &s.figure {
                    for p in figure_problems(f) {
                        err(format!("figure: {p}"));
                    }
                }
                loaded.push((path.clone(), s));
            }
        }
        report.diagnostics.extend(sub.diagnostics);
    }
    for (path, id, m) in diags {
        report.push(Level::Error, &path, id.as_deref(), m);
    }
    report.stimuli = loaded;
}

/// The subject a stimulus belongs to: the name of the file it is in.
pub fn stimulus_subject(path: &Path) -> String {
    path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string()
}

/// The numbers of the `{{n}}` markers in a drop-down prompt, in order of
/// appearance.
pub fn blank_markers(prompt: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let mut rest = prompt;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else { break };
        if let Ok(n) = after[..end].trim().parse::<usize>() {
            out.push(n);
        }
        rest = &after[end + 2..];
    }
    out
}

fn distinct(items: &[String]) -> bool {
    let set: BTreeSet<&str> = items.iter().map(|c| c.trim()).collect();
    set.len() == items.len() && items.iter().all(|i| !i.trim().is_empty())
}

/// Problems with a question's answer format, as messages (empty when fine).
pub fn format_problems(q: &Question) -> Vec<String> {
    let mut errs = Vec::new();
    let mut only = |name: &str, used: bool, allowed: bool| {
        if used && !allowed {
            errs.push(format!("{name} does not belong on a {} question", q.kind.name()));
        }
    };
    only("choices", !q.choices.is_empty(), q.kind == QuestionType::MultipleChoice);
    only("accept", !q.accept.is_empty(), q.kind == QuestionType::FillIn);
    only("blanks", !q.blanks.is_empty(), q.kind == QuestionType::Dropdown);
    only("items", !q.items.is_empty(), q.kind == QuestionType::Order);
    only("groups", !q.groups.is_empty(), q.kind == QuestionType::Sort);
    match q.kind {
        QuestionType::MultipleChoice => {
            if q.choices.len() < 2 {
                errs.push("needs at least two choices".into());
            }
            if !distinct(&q.choices) {
                errs.push("choices must be distinct".into());
            }
            if !q.choices.iter().any(|c| c.trim() == q.answer.trim()) {
                errs.push(format!("answer {:?} is not one of the choices", q.answer));
            }
        }
        QuestionType::FillIn => {
            if q.answer.trim().is_empty() {
                errs.push("answer must not be empty".into());
            }
            if q.accept.iter().any(|a| a.trim().is_empty()) {
                errs.push("accept entries must not be empty".into());
            }
        }
        QuestionType::Dropdown => {
            let markers = blank_markers(&q.prompt);
            let want: Vec<usize> = (1..=q.blanks.len()).collect();
            if q.blanks.is_empty() {
                errs.push("a dropdown question needs at least one entry in blanks".into());
            } else if markers != want {
                errs.push(format!(
                    "the prompt must hold the markers {{{{1}}}} to {{{{{}}}}} once each, in order, one per blank",
                    q.blanks.len()
                ));
            }
            for (i, b) in q.blanks.iter().enumerate() {
                if b.choices.len() < 2 || !distinct(&b.choices) {
                    errs.push(format!("blank {} needs at least two distinct choices", i + 1));
                }
                if !b.choices.iter().any(|c| c.trim() == b.answer.trim()) {
                    errs.push(format!("blank {} answer {:?} is not one of its choices", i + 1, b.answer));
                }
            }
        }
        QuestionType::Order => {
            if q.items.len() < 3 || !distinct(&q.items) {
                errs.push("an order question needs at least three distinct items".into());
            }
        }
        QuestionType::Sort => {
            if q.groups.len() < 2 {
                errs.push("a sort question needs at least two groups".into());
            }
            let names: Vec<String> = q.groups.iter().map(|g| g.name.clone()).collect();
            if !distinct(&names) {
                errs.push("group names must be distinct".into());
            }
            let all: Vec<String> = q.groups.iter().flat_map(|g| g.items.iter().cloned()).collect();
            if all.len() < 3 || !distinct(&all) {
                errs.push("a sort question needs at least three items, each in exactly one group".into());
            }
        }
    }
    if q.kind != QuestionType::MultipleChoice && q.kind != QuestionType::FillIn && !q.answer.trim().is_empty() {
        errs.push(format!("answer does not belong on a {} question", q.kind.name()));
    }
    errs
}

/// The id of the subject domain a question counts toward: the first part
/// of its topic code, or its skill's area for Math.
pub fn question_domain(q: &Question, skill: &Skill) -> String {
    match &q.topic {
        Some(t) => t.split('.').next().unwrap_or_default().to_string(),
        None => skill.area.clone(),
    }
}
