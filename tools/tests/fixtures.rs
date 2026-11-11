//! Runs the validator and answer checker over the fixture trees in
//! `tests/fixtures/`. Each tree gets the shared `skills.toml` and
//! `principles.toml` copied in, unless it has its own.

use std::path::{Path, PathBuf};

use ged_content::{Report, check_content};

fn tree(name: &str) -> Report {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures");
    let src = fixtures.join(name);
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("fixture-{name}"));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(out.join("math")).unwrap();
    for file in ["skills.toml", "principles.toml"] {
        let own = src.join(file);
        let from = if own.exists() { own } else { fixtures.join(file) };
        std::fs::copy(from, out.join(file)).unwrap();
    }
    if let Ok(entries) = std::fs::read_dir(src.join("math")) {
        for entry in entries {
            let p = entry.unwrap().path();
            std::fs::copy(&p, out.join("math").join(p.file_name().unwrap())).unwrap();
        }
    }
    check_content(&out)
}

#[test]
fn good_tree_passes() {
    let r = tree("good");
    for d in &r.diagnostics {
        eprintln!("{d}");
    }
    assert_eq!(r.errors(), 0);
    assert_eq!(r.warnings(), 0);
    assert_eq!(r.questions.len(), 3);
}

#[test]
fn bad_tree_reports_every_problem() {
    let r = tree("bad");
    for d in &r.diagnostics {
        eprintln!("{d}");
    }
    let expect = [
        "WRONG KEY",                       // q-ratios-percent-0001
        "unknown skill \"a-not-a-skill\"", // q-ratios-percent-0002
        "but lives in q-ratios-percent.toml",
        "same prompt as q-ratios-percent-0004",
        "is not one of the choices",       // 0005
        "check expression",                // 0006 broken expression
        "(four digits)",                   // bad id format
        "outside this skill's range",      // geometry difficulty 3
        "is not a number; add answer_value",
        "\"q-not-real\" is not a skill id",
        "invalid TOML",                    // unknown field `hint`
        "unknown principle \"Q.9.z\"",     // geometry 0003
        "principle Q.3.a belongs to skill \"q-ratios-percent\"", // geometry 0004
    ];
    for needle in expect {
        assert!(r.mentions(needle), "expected a diagnostic containing {needle:?}");
    }
}

#[test]
fn bad_principles_reported() {
    let r = tree("badprinciples");
    for d in &r.diagnostics {
        eprintln!("{d}");
    }
    let expect = [
        "duplicate principle id",                               // Q.3.a twice
        "not listed in any skill's targets",                    // Q.9.z
        "skills.toml lists it under \"q-geometry-2d\"",          // Q.4.a wrong skill
        "lesson must not be empty",                             // Q.4.a
        "principles.toml has no principle for it",              // Q.4.b missing
    ];
    for needle in expect {
        assert!(r.mentions(needle), "expected a diagnostic containing {needle:?}");
    }
}

#[test]
fn real_principles_cover_every_target() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("content");
    let r = check_content(&root);
    assert_eq!(r.errors(), 0, "cargo content should pass");
    assert_eq!(r.principles.len(), 62);
    assert!(r.questions.iter().all(|(_, q)| !q.principle.is_empty()));
}

#[test]
fn real_skill_map_is_valid() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("content");
    let skills = ged_content::load_skills(&root).expect("content/skills.toml should be valid");
    assert_eq!(skills.len(), 13);
}
