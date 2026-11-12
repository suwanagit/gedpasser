//! ged-content — validate the question bank, prepare review notes, and
//! approve reviewed items.
//!
//!   ged-content [check] [DIR]           validate everything (cargo content)
//!   ged-content template SKILL [DIR]    review-note skeleton for SKILL (cargo content-template SKILL)
//!                                       SKILL may also be `strategy` or `stimuli-<subject>`
//!   ged-content approve SKILL [DIR]     mark passed items reviewed  (cargo content-approve SKILL)
//!   ged-content demo OUT_DIR           real content plus fixtures/demo-science, for testing
//!   ged-content quiz SKILL [DIR]        questions without answers, for blind review (cargo content-quiz SKILL)
//!
//! DIR defaults to `content`. Exit code 1 on any error.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

use ged_content::{Status, Verdict, check_content};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = |i: usize| args.get(i).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("content"));
    match args.first().map(String::as_str) {
        Some("template") => match args.get(1) {
            Some(skill) => match ged_content::template(&dir(2), skill) {
                Ok(text) => {
                    print!("{text}");
                    ExitCode::SUCCESS
                }
                Err(e) => fail(&e),
            },
            None => fail("usage: ged-content template SKILL [DIR]"),
        },
        Some("approve") => match args.get(1) {
            Some(skill) => match ged_content::approve(&dir(2), skill) {
                Ok(ids) if ids.is_empty() => {
                    println!("Nothing to approve in {skill}: no drafts with a current pass review.");
                    ExitCode::SUCCESS
                }
                Ok(ids) => {
                    println!("Approved {} item(s) in {skill}: {}", ids.len(), ids.join(", "));
                    ExitCode::SUCCESS
                }
                Err(e) => fail(&e),
            },
            None => fail("usage: ged-content approve SKILL [DIR]"),
        },
        Some("quiz") => match args.get(1) {
            Some(skill) => quiz(&dir(2), skill),
            None => fail("usage: ged-content quiz SKILL [DIR]"),
        },
        Some("demo") => match args.get(1) {
            Some(out) => {
                let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
                match ged_content::demo::build(&root.join("content"), &root.join("fixtures").join("demo-science"), &PathBuf::from(out)) {
                    Ok(()) => {
                        println!("Built a demo bank in {out}. Run the server with GED_CONTENT={out} and GED_SERVE_DRAFTS=1.");
                        check(PathBuf::from(out))
                    }
                    Err(e) => fail(&e),
                }
            }
            None => fail("usage: ged-content demo OUT_DIR"),
        },
        Some("check") => check(dir(1)),
        Some(other) if other.starts_with('-') => fail("usage: ged-content [check|template|approve] ..."),
        Some(_) => check(dir(0)),
        None => check(dir(0)),
    }
}

/// Print a skill's draft questions WITHOUT answers, for blind solving by a
/// reviewer: id, difficulty, prompt and choices only.
fn quiz(root: &std::path::Path, skill: &str) -> ExitCode {
    let report = check_content(root);
    if !report.skills.iter().any(|s| s.id == skill) {
        return fail(&format!("unknown skill {skill:?}"));
    }
    println!("# Blind quiz for {skill}: solve each one BEFORE opening the question file.\n");
    let mut shown = std::collections::HashSet::new();
    for (_, q) in report.questions.iter().filter(|(_, q)| q.skill == skill && q.status != Status::Retired) {
        if let Some(st) = report.stimulus_of(q) {
            if shown.insert(st.id.clone()) {
                println!("### Stimulus {}{}", st.id, st.title.as_ref().map(|t| format!(": {t}")).unwrap_or_default());
                if !st.text.trim().is_empty() {
                    println!("{}", st.text.trim());
                }
                if let Some(t) = &st.table {
                    println!("| {} |", t.headers.join(" | "));
                    for row in &t.rows {
                        println!("| {} |", row.join(" | "));
                    }
                }
                if let Some(f) = &st.figure {
                    println!("[figure: {f:?}]");
                }
                println!("Source: {}\n", st.source.trim());
            }
        }
        println!("## {} (difficulty {}, {})", q.id, q.difficulty, q.kind.name());
        if let Some(id) = &q.stimulus {
            println!("(uses stimulus {id})");
        }
        println!("{}", q.prompt.trim());
        if let Some(f) = &q.figure {
            println!("[figure shown with the question: {f:?}]");
        }
        match q.kind {
            ged_content::QuestionType::MultipleChoice => {
                for (i, c) in q.choices.iter().enumerate() {
                    println!("  {}. {c}", i + 1);
                }
            }
            ged_content::QuestionType::FillIn => println!("(type your answer)"),
            ged_content::QuestionType::Dropdown => {
                for (i, b) in q.blanks.iter().enumerate() {
                    println!("  menu {}: {}", i + 1, b.choices.join(" | "));
                }
            }
            ged_content::QuestionType::Order => {
                // Alphabetical, so the listing does not give the order away.
                let mut items: Vec<&String> = q.items.iter().collect();
                items.sort();
                println!("(put in order)");
                for i in items {
                    println!("  - {i}");
                }
            }
            ged_content::QuestionType::Sort => {
                let mut items: Vec<&String> = q.groups.iter().flat_map(|g| g.items.iter()).collect();
                items.sort();
                println!("(sort into: {})", q.groups.iter().map(|g| g.name.as_str()).collect::<Vec<_>>().join(" | "));
                for i in items {
                    println!("  - {i}");
                }
            }
        }
        println!();
    }
    ExitCode::SUCCESS
}

fn fail(msg: &str) -> ExitCode {
    eprintln!("error: {msg}");
    ExitCode::FAILURE
}

fn check(root: PathBuf) -> ExitCode {
    let report = check_content(&root);

    let mut diags = report.diagnostics.clone();
    diags.sort_by(|a, b| b.level.cmp(&a.level).then(a.file.cmp(&b.file)).then(a.id.cmp(&b.id)));
    for d in &diags {
        println!("{d}");
    }

    // Per-skill table: reviewed / draft questions, and rejected drafts.
    let mut rows: BTreeMap<usize, (String, usize, usize, usize, usize, usize)> = BTreeMap::new();
    for (i, s) in report.skills.iter().enumerate() {
        rows.insert(i, (s.id.clone(), 0, 0, 0, 0, 0));
    }
    let index = |skill: &str| report.skills.iter().position(|s| s.id == skill);
    for (_, q) in &report.questions {
        if let Some(i) = index(&q.skill) {
            let row = rows.get_mut(&i).unwrap();
            match q.status {
                Status::Reviewed => row.1 += 1,
                Status::Draft => {
                    row.2 += 1;
                    let latest = ged_content::review::report_latest(&report.reviews, ged_content::ReviewKind::Question, &q.id);
                    if latest.is_some_and(|r| r.verdict == Verdict::Reject) {
                        row.3 += 1;
                    }
                }
                Status::Retired => {}
            }
        }
    }
    for p in &report.principles {
        if let Some(i) = index(&p.skill) {
            let row = rows.get_mut(&i).unwrap();
            if p.status == Status::Reviewed {
                row.4 += 1;
            }
            row.5 += 1;
        }
    }
    if !report.questions.is_empty() {
        println!("\n{:<20} {:>8} {:>6} {:>9} {:>8}", "skill", "reviewed", "draft", "rejected", "lessons");
        for (_, (id, rev, draft, rej, lrev, ltotal)) in &rows {
            println!("{id:<20} {rev:>8} {draft:>6} {rej:>9} {:>8}", format!("{lrev}/{ltotal}"));
        }
    }

    let count = |s: Status| report.questions.iter().filter(|(_, q)| q.status == s).count();
    let lessons_reviewed = report.principles.iter().filter(|p| p.status == Status::Reviewed).count();
    println!(
        "\n{}{} skills, {} principles ({} lessons reviewed, {} cards / {} reviewed), {} questions ({} reviewed, {} draft, {} retired), {} review notes: {} error(s), {} warning(s)",
        if report.stimuli.is_empty() { String::new() } else { format!("{} stimuli, ", report.stimuli.len()) },
        report.skills.len(),
        report.principles.len(),
        lessons_reviewed,
        report.cards.len(),
        report.cards.iter().filter(|(_, c)| c.status == Status::Reviewed).count(),
        report.questions.len(),
        count(Status::Reviewed),
        count(Status::Draft),
        count(Status::Retired),
        report.reviews.len(),
        report.errors(),
        report.warnings(),
    );

    if report.errors() > 0 { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}
