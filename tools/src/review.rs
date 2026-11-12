//! Content hashes, the review gate, review-note templates and approval.
//!
//! A question or lesson may only have `status = "reviewed"` if the latest
//! entry for it in `content/reviews/*.toml` is a `pass` for its *current*
//! content hash. `approve` is the only supported way to set that status.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::schema::{
    Card, Lesson, Principle, Question, QuestionType, Review, ReviewFile, ReviewKind, Status, StrategyFile, Verdict,
};
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

fn type_name(t: QuestionType) -> &'static str {
    match t {
        QuestionType::MultipleChoice => "multiple_choice",
        QuestionType::FillIn => "fill_in",
        QuestionType::Dropdown => "dropdown",
        QuestionType::DragDrop => "drag_drop",
    }
}

/// Hash of everything a reviewer checks in a question (not its status).
pub fn question_hash(q: &Question) -> String {
    let answer_value = q.answer_value.map(|v| v.to_string()).unwrap_or_default();
    let difficulty = q.difficulty.to_string();
    let mut parts = vec![
        "question",
        &q.skill,
        &q.principle,
        &difficulty,
        type_name(q.kind),
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
    // Only questions with a figure hash it, so older hashes stay valid.
    if let Some(f) = &q.figure {
        parts.push(format!("{f:?}"));
    }
    let refs: Vec<&str> = parts.iter().map(String::as_str).collect();
    hash_parts(&refs)
}

fn choices_joined(q: &Question) -> String {
    q.choices.join("\u{1f}")
}

/// Hash of everything a reviewer checks in a lesson (not its status).
pub fn lesson_hash(p: &Principle) -> String {
    hash_parts(&["lesson", &p.id, &p.skill, p.name.trim(), p.summary.trim(), p.lesson.trim()])
}

/// Hash of a full lesson (not its status).
pub fn unit_hash(l: &Lesson) -> String {
    let mut copy = l.clone();
    copy.status = Status::Draft;
    hash_parts(&["unit", &format!("{copy:?}")])
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
        if !skill_ids.contains(&stem.as_str()) && stem != "strategy" {
            diags.push((Level::Error, path.clone(), None, format!("review file {stem:?} is not named after a skill")));
        }
        let mut sub = Report::default();
        if let Some(file) = read_toml::<ReviewFile>(&path, &mut sub) {
            loaded.extend(file.review.into_iter().map(|r| (path.clone(), r)));
        }
        report.diagnostics.extend(sub.diagnostics);
    }

    let question_ids: HashMap<&str, &Question> = report.questions.iter().map(|(_, q)| (q.id.as_str(), q)).collect();
    let principle_ids: HashMap<&str, &Principle> = report.principles.iter().map(|p| (p.id.as_str(), p)).collect();
    for (path, r) in &loaded {
        let known = match r.kind {
            ReviewKind::Question => question_ids.contains_key(r.id.as_str()),
            ReviewKind::Lesson => principle_ids.contains_key(r.id.as_str()) || report.lessons.iter().any(|(_, l)| l.id == r.id),
            ReviewKind::Card => report.cards.iter().any(|(_, c)| c.id == r.id),
            ReviewKind::Strategy => r.id == "strategy" && report.strategy.is_some(),
        };
        if !known {
            diags.push((Level::Error, path.clone(), Some(r.id.clone()), "review refers to an unknown item".into()));
        }
        if r.note.trim().is_empty() {
            diags.push((Level::Error, path.clone(), Some(r.id.clone()), "review note must not be empty".into()));
        }
        if r.kind == ReviewKind::Question && r.verdict == Verdict::Pass && r.solved.as_deref().unwrap_or("").trim().is_empty()
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
        gate(report_latest(&report.reviews, ReviewKind::Question, &q.id), q.status, &question_hash(q), path, &q.id, &mut diags);
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
    for (path, l) in &report.lessons {
        gate(report_latest(&report.reviews, ReviewKind::Lesson, &l.id), l.status, &unit_hash(l), path, &l.id, &mut diags);
    }
    for (path, c) in &report.cards {
        gate(report_latest(&report.reviews, ReviewKind::Card, &c.id), c.status, &card_hash(c), path, &c.id, &mut diags);
    }
    if let Some(s) = &report.strategy {
        let path = root.join("strategy.toml");
        gate(report_latest(&report.reviews, ReviewKind::Strategy, "strategy"), s.status, &strategy_hash(s), &path, "strategy", &mut diags);
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
    report_latest(&report.reviews, kind, id).is_some_and(|r| r.verdict == Verdict::Pass && r.hash == hash)
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
    if !report.skills.iter().any(|s| s.id == skill) {
        return Err(format!("unknown skill {skill:?}"));
    }
    let mut out = format!(
        "# Review notes for {skill}. Append these entries to content/reviews/{skill}.toml.\n\
         # verdict: pass | fix | reject. For questions, fill `solved` with your own\n\
         # answer worked out BEFORE reading the answer key.\n"
    );
    let mut push = |kind: &str, id: &str, hash: &str| {
        let solved = if kind == "question" { "solved = \"\"\n" } else { "" };
        out.push_str(&format!(
            "\n[[review]]\nid = \"{id}\"\nkind = \"{kind}\"\nhash = \"{hash}\"\nverdict = \"\"\n{solved}note = \"\"\ndate = \"\"\n"
        ));
    };
    let mut count = 0;
    for (_, q) in report.questions.iter().filter(|(_, q)| q.skill == skill && q.status != Status::Retired) {
        let h = question_hash(q);
        if !has_current_pass(&report, ReviewKind::Question, &q.id, &h) {
            push("question", &q.id, &h);
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
    for (_, l) in report.lessons.iter().filter(|(_, l)| skill_principles.contains(&l.principle.as_str()) && l.status != Status::Retired) {
        let h = unit_hash(l);
        if !has_current_pass(&report, ReviewKind::Lesson, &l.id, &h) {
            push("lesson", &l.id, &h);
            count += 1;
        }
    }
    for (_, c) in report.cards.iter().filter(|(_, c)| skill_principles.contains(&c.id.as_str()) && c.status != Status::Retired) {
        let h = card_hash(c);
        if !has_current_pass(&report, ReviewKind::Card, &c.id, &h) {
            push("card", &c.id, &h);
            count += 1;
        }
    }
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
    if !report.skills.iter().any(|s| s.id == skill) {
        return Err(format!("unknown skill {skill:?}"));
    }
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

    let lfile = root.join("lessons").join(format!("{skill}.toml"));
    let ids: Vec<String> = report
        .lessons
        .iter()
        .filter(|(p, l)| *p == lfile && l.status == Status::Draft)
        .filter(|(_, l)| has_current_pass(&report, ReviewKind::Lesson, &l.id, &unit_hash(l)))
        .map(|(_, l)| l.id.clone())
        .collect();
    if !ids.is_empty() {
        set_status(&lfile, "[[lesson]]", &ids)?;
        approved.extend(ids);
    }

    let qfile = root.join("math").join(format!("{skill}.toml"));
    let ids: Vec<String> = report
        .questions
        .iter()
        .filter(|(_, q)| q.skill == skill && q.status == Status::Draft)
        .filter(|(_, q)| has_current_pass(&report, ReviewKind::Question, &q.id, &question_hash(q)))
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
