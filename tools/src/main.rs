//! `ged-content [CONTENT_DIR]` — validate the question bank and check every
//! answer. Exits 1 if there is any error. Run it with `cargo content`.

use std::path::PathBuf;
use std::process::ExitCode;

use ged_content::{Status, check_content};

fn main() -> ExitCode {
    let root = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("content"));
    let report = check_content(&root);

    let mut diags = report.diagnostics.clone();
    diags.sort_by(|a, b| b.level.cmp(&a.level).then(a.file.cmp(&b.file)).then(a.id.cmp(&b.id)));
    for d in &diags {
        println!("{d}");
    }

    let count = |s: Status| report.questions.iter().filter(|(_, q)| q.status == s).count();
    println!(
        "\n{} skills, {} questions ({} reviewed, {} draft, {} retired): {} error(s), {} warning(s)",
        report.skills.len(),
        report.questions.len(),
        count(Status::Reviewed),
        count(Status::Draft),
        count(Status::Retired),
        report.errors(),
        report.warnings(),
    );

    if report.errors() > 0 { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}
