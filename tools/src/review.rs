//! Content hashes, the review gate, review-note templates and approval.
//!
//! A question or lesson may only have `status = "reviewed"` if the latest
//! entry for it in `content/reviews/*.toml` is a `pass` for its *current*
//! content hash. `approve` is the only supported way to set that status.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::schema::{
    Card, Principle, Question, Review, ReviewFile, ReviewKind, ReviewRole, Status, Stimulus, StrategyFile, StudyFile, Verdict,
};
use crate::subjects::stimulus_subject;
use crate::{Level, Report, check_content, read_toml};

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hash_parts(parts: &[&str]) -> String {
    let mut h = Sha256::new();
    for p in parts {
        h.update(p.as_bytes());
        h.update([0u8]);
    }
    hex(&h.finalize())[..16].to_string()
}

/// Hash of everything a reviewer checks in a question (not its status).
/// `stimulus` is the stimulus the question points to, so that editing a
/// passage sends its questions back for review.
pub fn question_hash(q: &Question, stimulus: Option<&Stimulus>) -> String {
    let answer_value = q.answer_value.map(|v| v.to_string()).unwrap_or_default();
    let difficulty = q.difficulty.to_string();
    let mut parts = vec![
        "question",
        &q.skill,
        &q.principle,
        &difficulty,
        q.kind.name(),
        q.prompt.trim(),
        &choices_joined(q),
        q.answer.trim(),
        &answer_value,
        q.check.trim(),
        q.explanation.trim(),
        q.common_mistake.trim(),
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<Vec<String>>();
    // Newer fields are hashed only when used, so older hashes stay valid.
    if let Some(f) = &q.figure {
        parts.push(format!("{f:?}"));
    }
    if !q.accept.is_empty() {
        parts.push(format!("accept {:?}", q.accept));
    }
    if !q.blanks.is_empty() {
        parts.push(format!("{:?}", q.blanks));
    }
    if !q.items.is_empty() {
        parts.push(format!("items {:?}", q.items));
    }
    if !q.groups.is_empty() {
        parts.push(format!("{:?}", q.groups));
    }
    if let Some(t) = &q.topic {
        parts.push(format!("topic {t}"));
    }
    if !q.sources.is_empty() {
        parts.push(format!("sources {:?}", q.sources));
    }
    if let Some(id) = &q.stimulus {
        parts.push(format!("stimulus {id} {}", stimulus.map(stimulus_hash).unwrap_or_default()));
    }
    let refs: Vec<&str> = parts.iter().map(String::as_str).collect();
    hash_parts(&refs)
}

/// Hash of a shared stimulus (not its status).
pub fn stimulus_hash(s: &Stimulus) -> String {
    let table = s.table.as_ref().map(|t| format!("{t:?}")).unwrap_or_default();
    let figure = s.figure.as_ref().map(|f| format!("{f:?}")).unwrap_or_default();
    hash_parts(&["stimulus", &s.id, s.title.as_deref().unwrap_or("").trim(), s.text.trim(), &table, &figure, s.source.trim()])
}

fn choices_joined(q: &Question) -> String {
    q.choices.join("\u{1f}")
}

/// Hash of everything a reviewer checks in a lesson (not its status).
pub fn lesson_hash(p: &Principle) -> String {
    hash_parts(&["lesson", &p.id, &p.skill, p.name.trim(), p.summary.trim(), p.lesson.trim()])
}

/// Hash of a short lesson card (not its status).
pub fn card_hash(c: &Card) -> String {
    let figure = c.figure.as_ref().map(|f| format!("{f:?}")).unwrap_or_default();
    hash_parts(&["card", &c.id, c.title.trim(), c.key.trim(), &c.steps.join("\u{1f}"), c.example.trim(), c.hack.trim(), &figure])
}

/// Hash of the strategy file's content (not its status).
pub fn strategy_hash(s: &StrategyFile) -> String {
    hash_parts(&[
        "strategy",
        &format!("{:?}", s.high_yield),
        &format!("{:?}", s.medium_yield),
        &format!("{:?}", s.tactic),
        &format!("{:?}", s.formula),
    ])
}

/// Hash of the study file's content (not its status).
pub fn study_hash(s: &StudyFile) -> String {
    hash_parts(&["study", &format!("{:?}", s.topic)])
}

/// Load `root/reviews/*.toml` and apply the review gate to every item.
pub(crate) fn check_reviews(root: &Path, report: &mut Report) {
    let dir = root.join("reviews");
    let mut paths: Vec<PathBuf> = match std::fs::read_dir(&dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect(),
        Err(_) => Vec::new(), // no reviews yet
    };
    paths.sort();
    let skill_ids: Vec<&str> = report.skills.iter().map(|s| s.id.as_str()).collect();
    let mut loaded = Vec::new();
    let mut diags = Vec::new();
    for path in paths {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
        let stimuli_file = stem.strip_prefix("stimuli-").is_some_and(|sub| report.subjects.iter().any(|s| s.id == sub));
        if !skill_ids.contains(&stem.as_str()) && stem != "strategy" && stem != "study" && !stimuli_file {
            diags.push((Level::Error, path.clone(), None, format!("review file {stem:?} is not named after a skill")));
        }
        let mut sub = Report::default();
        if let Some(file) = read_toml::<ReviewFile>(&path, &mut sub) {
            loaded.extend(file.review.into_iter().map(|r| (path.clone(), r)));
        }
        report.diagnostics.extend(sub.diagnostics);
    }

    let question_ids: HashMap<&str, &Question> = report.questions.iter().map(|(_, q)| (q.id.as_str(), q)).collect();
    let panel_ids: std::collections::HashSet<String> = panel_items(report).into_iter().collect();
    let principle_ids: HashMap<&str, &Principle> = report.principles.iter().map(|p| (p.id.as_str(), p)).collect();
    for (path, r) in &loaded {
        let known = match r.kind {
            ReviewKind::Question => question_ids.contains_key(r.id.as_str()),
            ReviewKind::Lesson => principle_ids.contains_key(r.id.as_str()),
            ReviewKind::Card => report.cards.iter().any(|(_, c)| c.id == r.id),
            ReviewKind::Strategy => r.id == "strategy" && report.strategy.is_some(),
            ReviewKind::Study => r.id == "study" && report.study.is_some(),
            ReviewKind::Stimulus => report.stimulus(&r.id).is_some(),
        };
        if !known {
            diags.push((Level::Error, path.clone(), Some(r.id.clone()), "review refers to an unknown item".into()));
        }
        if r.note.trim().is_empty() {
            diags.push((Level::Error, path.clone(), Some(r.id.clone()), "review note must not be empty".into()));
        }
        let on_panel = panel_ids.contains(&panel_key(r.kind, &r.id));
        if on_panel && (r.reviewer.as_deref().unwrap_or("").trim().is_empty() || r.role.is_none()) {
            diags.push((
                Level::Error,
                path.clone(),
                Some(r.id.clone()),
                "non-math reviews need `reviewer` (a name) and `role` (\"solver\" or \"factcheck\")".into(),
            ));
        }
        if r.kind == ReviewKind::Question
            && r.verdict == Verdict::Pass
            && r.role != Some(ReviewRole::Factcheck)
            && r.solved.as_deref().unwrap_or("").trim().is_empty()
        {
            diags.push((
                Level::Error,
                path.clone(),
                Some(r.id.clone()),
                "a passed question review must record the reviewer's own answer in `solved`".into(),
            ));
        }
    }
    report.reviews = loaded;

    for (path, q) in &report.questions {
        let hash = question_hash(q, report.stimulus_of(q));
        if panel_ids.contains(&panel_key(ReviewKind::Question, &q.id)) {
            panel_gate(&report.reviews, ReviewKind::Question, q.status, &hash, path, &q.id, &mut diags);
        } else {
            gate(report_latest(&report.reviews, ReviewKind::Question, &q.id), q.status, &hash, path, &q.id, &mut diags);
        }
    }
    for (path, st) in &report.stimuli {
        panel_gate(&report.reviews, ReviewKind::Stimulus, st.status, &stimulus_hash(st), path, &st.id, &mut diags);
    }
    let principles_path = root.join("principles.toml");
    for p in &report.principles {
        gate(
            report_latest(&report.reviews, ReviewKind::Lesson, &p.id),
            p.status,
            &lesson_hash(p),
            &principles_path,
            &p.id,
            &mut diags,
        );
    }
    for (path, c) in &report.cards {
        gate(report_latest(&report.reviews, ReviewKind::Card, &c.id), c.status, &card_hash(c), path, &c.id, &mut diags);
    }
    if let Some(s) = &report.strategy {
        let path = root.join("strategy.toml");
        gate(report_latest(&report.reviews, ReviewKind::Strategy, "strategy"), s.status, &strategy_hash(s), &path, "strategy", &mut diags);
    }
    if let Some(s) = &report.study {
        let path = root.join("study.toml");
        gate(report_latest(&report.reviews, ReviewKind::Study, "study"), s.status, &study_hash(s), &path, "study", &mut diags);
    }
    for (level, path, id, m) in diags {
        report.diagnostics.push(crate::Diagnostic { level, file: path, id, message: m });
    }
}

/// The latest review for an item, if any.
pub fn report_latest<'a>(reviews: &'a [(PathBuf, Review)], kind: ReviewKind, id: &str) -> Option<&'a Review> {
    reviews.iter().rev().map(|(_, r)| r).find(|r| r.kind == kind && r.id == id)
}

type Diag = (Level, PathBuf, Option<String>, String);

fn gate(latest: Option<&Review>, status: Status, hash: &str, path: &Path, id: &str, diags: &mut Vec<Diag>) {
    let current_pass = latest.is_some_and(|r| r.verdict == Verdict::Pass && r.hash == hash);
    if status == Status::Reviewed && !current_pass {
        let why = match latest {
            None => "it has no review".to_string(),
            Some(r) if r.hash != hash => "it changed after it was reviewed; it needs a fresh review".to_string(),
            Some(r) => format!("its latest review verdict is {:?}", r.verdict).to_lowercase(),
        };
        diags.push((
            Level::Error,
            path.to_path_buf(),
            Some(id.to_string()),
            format!("marked reviewed but {why} (only `ged-content approve` may set reviewed)"),
        ));
    }
    if status == Status::Draft && latest.is_some_and(|r| r.verdict == Verdict::Reject && r.hash == hash) {
        diags.push((
            Level::Warning,
            path.to_path_buf(),
            Some(id.to_string()),
            "reviewer rejected this draft; remove or rewrite it".into(),
        ));
    }
}

/// True if the item has a current pass and so needs no review.
fn has_current_pass(report: &Report, kind: ReviewKind, id: &str, hash: &str) -> bool {
    if report_panel_item(report, kind, id) {
        return panel(&report.reviews, kind, id, hash).done(kind);
    }
    report_latest(&report.reviews, kind, id).is_some_and(|r| r.verdict == Verdict::Pass && r.hash == hash)
}

// ── Non-math review panel ─────────────────────────────────────────────────
//
// Content without a code check needs several independent reviewers: two
// blind solvers and a fact-checker for a question, one solver and a
// fact-checker for a stimulus. Each reviewer's latest entry counts once.

fn panel_key(kind: ReviewKind, id: &str) -> String {
    format!("{kind:?}:{id}")
}

/// Keys of every item that needs the panel: non-math questions, and stimuli.
fn panel_items(report: &Report) -> Vec<String> {
    let subject: HashMap<&str, &str> = report.skills.iter().map(|s| (s.id.as_str(), s.subject.as_str())).collect();
    let mut out: Vec<String> = report
        .questions
        .iter()
        .filter(|(_, q)| subject.get(q.skill.as_str()).is_some_and(|s| *s != "math"))
        .map(|(_, q)| panel_key(ReviewKind::Question, &q.id))
        .collect();
    out.extend(report.stimuli.iter().map(|(_, s)| panel_key(ReviewKind::Stimulus, &s.id)));
    out
}

fn report_panel_item(report: &Report, kind: ReviewKind, id: &str) -> bool {
    let key = panel_key(kind, id);
    panel_items(report).contains(&key)
}

/// Passes needed: (solvers, fact-checkers).
pub fn panel_needs(kind: ReviewKind) -> (usize, usize) {
    match kind {
        ReviewKind::Question => (2, 1),
        _ => (1, 1),
    }
}

/// Where an item stands with its panel, for its current content.
pub struct Panel<'a> {
    pub solvers: usize,
    pub factchecks: usize,
    /// A current fix or reject from any reviewer.
    pub blocking: Option<&'a Review>,
    /// Some reviewer passed an older version.
    pub stale: bool,
}

impl Panel<'_> {
    pub fn done(&self, kind: ReviewKind) -> bool {
        let (s, f) = panel_needs(kind);
        self.blocking.is_none() && self.solvers >= s && self.factchecks >= f
    }
}

pub fn panel<'a>(reviews: &'a [(PathBuf, Review)], kind: ReviewKind, id: &str, hash: &str) -> Panel<'a> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Panel { solvers: 0, factchecks: 0, blocking: None, stale: false };
    for r in reviews.iter().rev().map(|(_, r)| r).filter(|r| r.kind == kind && r.id == id) {
        let (Some(name), Some(role)) = (r.reviewer.as_deref(), r.role) else { continue };
        if !seen.insert(name.trim().to_lowercase()) {
            continue; // only each reviewer's latest entry counts
        }
        if r.hash != hash {
            out.stale = true;
            continue;
        }
        match (r.verdict, role) {
            (Verdict::Pass, ReviewRole::Solver) => out.solvers += 1,
            (Verdict::Pass, ReviewRole::Factcheck) => out.factchecks += 1,
            _ => {
                if out.blocking.is_none() {
                    out.blocking = Some(r);
                }
            }
        }
    }
    out
}

fn panel_gate(reviews: &[(PathBuf, Review)], kind: ReviewKind, status: Status, hash: &str, path: &Path, id: &str, diags: &mut Vec<Diag>) {
    let p = panel(reviews, kind, id, hash);
    let (need_s, need_f) = panel_needs(kind);
    if status == Status::Reviewed && !p.done(kind) {
        let why = match p.blocking {
            Some(r) => format!("{} gave verdict {:?}", r.reviewer.as_deref().unwrap_or("a reviewer"), r.verdict).to_lowercase(),
            None => format!(
                "it has {} of {need_s} solver passes and {} of {need_f} fact-check passes from different reviewers{}",
                p.solvers,
                p.factchecks,
                if p.stale { " (some reviews are for an older version)" } else { "" }
            ),
        };
        diags.push((
            Level::Error,
            path.to_path_buf(),
            Some(id.to_string()),
            format!("marked reviewed but {why} (only `ged-content approve` may set reviewed)"),
        ));
    }
    if status == Status::Draft && p.blocking.is_some_and(|r| r.verdict == Verdict::Reject) {
        diags.push((Level::Warning, path.to_path_buf(), Some(id.to_string()), "a reviewer rejected this draft; remove or rewrite it".into()));
    }
}

/// Template entries for the panel roles an item still needs.
fn panel_template(report: &Report, kind: ReviewKind, kind_name: &str, id: &str, hash: &str) -> String {
    let p = panel(&report.reviews, kind, id, hash);
    let (need_s, need_f) = panel_needs(kind);
    let mut out = String::new();
    let solved = if kind == ReviewKind::Question { "solved = \"\"\n" } else { "" };
    for _ in p.solvers..need_s {
        out.push_str(&format!(
            "\n[[review]]\nid = \"{id}\"\nkind = \"{kind_name}\"\nhash = \"{hash}\"\nreviewer = \"\"\nrole = \"solver\"\nverdict = \"\"\n{solved}note = \"\"\ndate = \"\"\n"
        ));
    }
    for _ in p.factchecks..need_f {
        out.push_str(&format!(
            "\n[[review]]\nid = \"{id}\"\nkind = \"{kind_name}\"\nhash = \"{hash}\"\nreviewer = \"\"\nrole = \"factcheck\"\nverdict = \"\"\nnote = \"\"\ndate = \"\"\n"
        ));
    }
    out
}

/// A review-note skeleton for every item in `skill` without a current pass:
/// its questions, and the lessons of its principles.
pub fn template(root: &Path, skill: &str) -> Result<String, String> {
    let report = check_content(root);
    if skill == "strategy" {
        let s = report.strategy.as_ref().ok_or("content/strategy.toml is missing or invalid")?;
        let h = strategy_hash(s);
        if has_current_pass(&report, ReviewKind::Strategy, "strategy", &h) {
            return Ok("# Nothing to review: strategy.toml has a current pass.\n".into());
        }
        return Ok(format!(
            "# Review notes for content/strategy.toml. Append to content/reviews/strategy.toml.\n\
             \n[[review]]\nid = \"strategy\"\nkind = \"strategy\"\nhash = \"{h}\"\nverdict = \"\"\nnote = \"\"\ndate = \"\"\n"
        ));
    }
    if skill == "study" {
        let s = report.study.as_ref().ok_or("content/study.toml is missing or invalid")?;
        let h = study_hash(s);
        if has_current_pass(&report, ReviewKind::Study, "study", &h) {
            return Ok("# Nothing to review: study.toml has a current pass.\n".into());
        }
        return Ok(format!(
            "# Review notes for content/study.toml. Append to content/reviews/study.toml.\n\
             \n[[review]]\nid = \"study\"\nkind = \"study\"\nhash = \"{h}\"\nverdict = \"\"\nnote = \"\"\ndate = \"\"\n"
        ));
    }
    if let Some(subject) = skill.strip_prefix("stimuli-") {
        let mut out = format!("# Review notes for the {subject} stimuli. Append to content/reviews/{skill}.toml.\n");
        let mut count = 0;
        for (path, st) in report.stimuli.iter().filter(|(p, st)| stimulus_subject(p) == subject && st.status != Status::Retired) {
            let _ = path;
            let h = stimulus_hash(st);
            if !has_current_pass(&report, ReviewKind::Stimulus, &st.id, &h) {
                out.push_str(&panel_template(&report, ReviewKind::Stimulus, "stimulus", &st.id, &h));
                count += 1;
            }
        }
        if count == 0 {
            out.push_str("\n# Nothing to review: every stimulus has a current pass.\n");
        }
        return Ok(out);
    }
    if !report.skills.iter().any(|s| s.id == skill) {
        return Err(format!("unknown skill {skill:?}"));
    }
    let mut out = format!(
        "# Review notes for {skill}. Append these entries to content/reviews/{skill}.toml.\n\
         # verdict: pass | fix | reject. For questions, fill `solved` with your own\n\
         # answer worked out BEFORE reading the answer key.\n"
    );
    let mut panel_out = String::new();
    let mut push = |kind: &str, id: &str, hash: &str| {
        let solved = if kind == "question" { "solved = \"\"\n" } else { "" };
        out.push_str(&format!(
            "\n[[review]]\nid = \"{id}\"\nkind = \"{kind}\"\nhash = \"{hash}\"\nverdict = \"\"\n{solved}note = \"\"\ndate = \"\"\n"
        ));
    };
    let mut count = 0;
    for (_, q) in report.questions.iter().filter(|(_, q)| q.skill == skill && q.status != Status::Retired) {
        let h = question_hash(q, report.stimulus_of(q));
        if !has_current_pass(&report, ReviewKind::Question, &q.id, &h) {
            if report_panel_item(&report, ReviewKind::Question, &q.id) {
                panel_out.push_str(&panel_template(&report, ReviewKind::Question, "question", &q.id, &h));
            } else {
                push("question", &q.id, &h);
            }
            count += 1;
        }
    }
    for p in report.principles.iter().filter(|p| p.skill == skill && p.status != Status::Retired) {
        let h = lesson_hash(p);
        if !has_current_pass(&report, ReviewKind::Lesson, &p.id, &h) {
            push("lesson", &p.id, &h);
            count += 1;
        }
    }
    let skill_principles: Vec<&str> = report.principles.iter().filter(|p| p.skill == skill).map(|p| p.id.as_str()).collect();
    for (_, c) in report.cards.iter().filter(|(_, c)| skill_principles.contains(&c.id.as_str()) && c.status != Status::Retired) {
        let h = card_hash(c);
        if !has_current_pass(&report, ReviewKind::Card, &c.id, &h) {
            push("card", &c.id, &h);
            count += 1;
        }
    }
    out.push_str(&panel_out);
    if count == 0 {
        out.push_str("\n# Nothing to review: every item in this skill has a current pass.\n");
    }
    Ok(out)
}

/// Set `status = "reviewed"` on every draft question and lesson in `skill`
/// whose latest review is a pass for its current content. Returns the ids
/// that were approved.
pub fn approve(root: &Path, skill: &str) -> Result<Vec<String>, String> {
    let report = check_content(root);
    if report.errors() > 0 {
        return Err(format!("fix the {} error(s) from `cargo content` first", report.errors()));
    }
    if skill == "strategy" {
        let s = report.strategy.as_ref().ok_or("content/strategy.toml is missing or invalid")?;
        if s.status != Status::Draft || !has_current_pass(&report, ReviewKind::Strategy, "strategy", &strategy_hash(s)) {
            return Ok(Vec::new());
        }
        let path = root.join("strategy.toml");
        let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let new = text.replacen("status = \"draft\"", "status = \"reviewed\"", 1);
        std::fs::write(&path, new).map_err(|e| e.to_string())?;
        return Ok(vec!["strategy".into()]);
    }
    if skill == "study" {
        let s = report.study.as_ref().ok_or("content/study.toml is missing or invalid")?;
        if s.status != Status::Draft || !has_current_pass(&report, ReviewKind::Study, "study", &study_hash(s)) {
            return Ok(Vec::new());
        }
        let path = root.join("study.toml");
        let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let new = text.replacen("status = \"draft\"", "status = \"reviewed\"", 1);
        std::fs::write(&path, new).map_err(|e| e.to_string())?;
        return Ok(vec!["study".into()]);
    }
    if let Some(subject) = skill.strip_prefix("stimuli-") {
        let path = root.join("stimuli").join(format!("{subject}.toml"));
        let ids: Vec<String> = report
            .stimuli
            .iter()
            .filter(|(p, st)| *p == path && st.status == Status::Draft)
            .filter(|(_, st)| has_current_pass(&report, ReviewKind::Stimulus, &st.id, &stimulus_hash(st)))
            .map(|(_, st)| st.id.clone())
            .collect();
        if !ids.is_empty() {
            set_status(&path, "[[stimulus]]", &ids)?;
        }
        let after = check_content(root);
        if after.errors() > 0 {
            return Err(format!("approval left {} error(s); run `cargo content`", after.errors()));
        }
        return Ok(ids);
    }
    let Some(subject) = report.skills.iter().find(|s| s.id == skill).map(|s| s.subject.clone()) else {
        return Err(format!("unknown skill {skill:?}"));
    };
    let mut approved = Vec::new();

    let cfile = root.join("cards").join(format!("{skill}.toml"));
    let ids: Vec<String> = report
        .cards
        .iter()
        .filter(|(p, c)| *p == cfile && c.status == Status::Draft)
        .filter(|(_, c)| has_current_pass(&report, ReviewKind::Card, &c.id, &card_hash(c)))
        .map(|(_, c)| c.id.clone())
        .collect();
    if !ids.is_empty() {
        set_status(&cfile, "[[card]]", &ids)?;
        approved.extend(ids);
    }

    let qfile = root.join(&subject).join(format!("{skill}.toml"));
    let ids: Vec<String> = report
        .questions
        .iter()
        .filter(|(_, q)| q.skill == skill && q.status == Status::Draft)
        .filter(|(_, q)| has_current_pass(&report, ReviewKind::Question, &q.id, &question_hash(q, report.stimulus_of(q))))
        // A question waits until its passage is approved.
        .filter(|(_, q)| report.stimulus_of(q).is_none_or(|st| st.status == Status::Reviewed))
        .map(|(_, q)| q.id.clone())
        .collect();
    if !ids.is_empty() {
        set_status(&qfile, "[[question]]", &ids)?;
        approved.extend(ids);
    }

    let pfile = root.join("principles.toml");
    let ids: Vec<String> = report
        .principles
        .iter()
        .filter(|p| p.skill == skill && p.status == Status::Draft)
        .filter(|p| has_current_pass(&report, ReviewKind::Lesson, &p.id, &lesson_hash(p)))
        .map(|p| p.id.clone())
        .collect();
    if !ids.is_empty() {
        set_status(&pfile, "[[principle]]", &ids)?;
        approved.extend(ids);
    }

    let after = check_content(root);
    if after.errors() > 0 {
        return Err(format!("approval left {} error(s); run `cargo content`", after.errors()));
    }
    Ok(approved)
}

/// Rewrite `status = "draft"` to `status = "reviewed"` inside the TOML
/// tables (starting with `header`) whose `id` is in `ids`. Everything else
/// in the file is left byte-for-byte unchanged.
fn set_status(path: &Path, header: &str, ids: &[String]) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let mut out = String::with_capacity(text.len() + ids.len() * 3);
    let mut current_id: Option<String> = None;
    let mut changed = 0;
    for line in text.split_inclusive('\n') {
        let t = line.trim_start();
        if t.starts_with(header) {
            current_id = None;
        } else if let Some(rest) = t.strip_prefix("id = \"") {
            current_id = rest.split('"').next().map(str::to_string);
        } else if t.starts_with("status = \"draft\"") && current_id.as_ref().is_some_and(|id| ids.contains(id)) {
            out.push_str(&line.replacen("status = \"draft\"", "status = \"reviewed\"", 1));
            changed += 1;
            continue;
        }
        out.push_str(line);
    }
    if changed != ids.len() {
        return Err(format!(
            "expected to approve {} item(s) in {} but found {} draft status line(s); put `status` after `id`",
            ids.len(),
            path.display(),
            changed
        ));
    }
    std::fs::write(path, out).map_err(|e| format!("cannot write {}: {e}", path.display()))
}
