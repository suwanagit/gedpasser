//! Runs the validator and answer checker over the fixture trees in
//! `tests/fixtures/`. Each tree gets the shared `skills.toml` copied in.

use std::path::{Path, PathBuf};

use ged_content::{Report, check_content};

fn tree(name: &str) -> Report {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures");
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("fixture-{name}"));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(out.join("math")).unwrap();
    std::fs::copy(fixtures.join("skills.toml"), out.join("skills.toml")).unwrap();
    for entry in std::fs::read_dir(fixtures.join(name).join("math")).unwrap() {
        let p = entry.unwrap().path();
        std::fs::copy(&p, out.join("math").join(p.file_name().unwrap())).unwrap();
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
    ];
    for needle in expect {
        assert!(r.mentions(needle), "expected a diagnostic containing {needle:?}");
    }
}

#[test]
fn real_skill_map_is_valid() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("content");
    let skills = ged_content::load_skills(&root).expect("content/skills.toml should be valid");
    assert_eq!(skills.len(), 13);
}
