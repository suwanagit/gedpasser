//! Runs the validator and answer checker over the fixture trees in
//! `tests/fixtures/`. Each tree gets the shared `skills.toml` and
//! `principles.toml` copied in, unless it has its own.

use std::path::{Path, PathBuf};

use ged_content::{Report, Status, approve, check_content, lesson_hash, question_hash, template};

fn tree(name: &str) -> Report {
    check_content(&copy_tree(name, name))
}

/// Copy fixture tree `name` to a fresh temp dir called `dest` and return it.
fn copy_tree(name: &str, dest: &str) -> PathBuf {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures");
    let src = fixtures.join(name);
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("fixture-{dest}"));
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
    out
}

fn write_reviews(root: &Path, skill: &str, entries: &[(&str, &str, &str, &str)]) {
    // (id, kind, hash, verdict)
    let mut text = String::new();
    for (id, kind, hash, verdict) in entries {
        text.push_str(&format!(
            "[[review]]\nid = \"{id}\"\nkind = \"{kind}\"\nhash = \"{hash}\"\nverdict = \"{verdict}\"\nsolved = \"x\"\nnote = \"checked\"\ndate = \"2026-11-10\"\n\n"
        ));
    }
    std::fs::create_dir_all(root.join("reviews")).unwrap();
    std::fs::write(root.join("reviews").join(format!("{skill}.toml")), text).unwrap();
}

fn question<'a>(r: &'a Report, id: &str) -> &'a ged_content::Question {
    &r.questions.iter().find(|(_, q)| q.id == id).unwrap().1
}

#[test]
fn approve_flips_only_current_passes() {
    let root = copy_tree("good", "approve");
    let r = check_content(&root);
    let h1 = question_hash(question(&r, "q-ratios-percent-0001"), None);
    let h2 = question_hash(question(&r, "q-ratios-percent-0002"), None);
    let h3 = question_hash(question(&r, "q-ratios-percent-0003"), None);
    let lesson = lesson_hash(r.principles.iter().find(|p| p.id == "Q.3.a").unwrap());
    write_reviews(
        &root,
        "q-ratios-percent",
        &[
            ("q-ratios-percent-0001", "question", &h1, "pass"),
            ("q-ratios-percent-0002", "question", &h2, "fix"),
            ("q-ratios-percent-0003", "question", "0000000000000000", "pass"), // stale hash
            ("q-ratios-percent-0002", "question", &h2, "fix"),
            ("Q.3.a", "lesson", &lesson, "pass"),
        ],
    );
    let _ = h3;
    let approved = approve(&root, "q-ratios-percent").expect("approve runs");
    assert_eq!(approved, vec!["q-ratios-percent-0001".to_string(), "Q.3.a".to_string()]);

    let r = check_content(&root);
    assert_eq!(r.errors(), 0, "{:?}", r.diagnostics);
    assert_eq!(question(&r, "q-ratios-percent-0001").status, Status::Reviewed);
    assert_eq!(question(&r, "q-ratios-percent-0002").status, Status::Draft);
    assert_eq!(question(&r, "q-ratios-percent-0003").status, Status::Draft);
    assert_eq!(r.principles.iter().find(|p| p.id == "Q.3.a").unwrap().status, Status::Reviewed);

    // Editing a reviewed question afterwards breaks the gate.
    let file = root.join("math").join("q-ratios-percent.toml");
    let text = std::fs::read_to_string(&file).unwrap().replace("packs 240 kg", "packs 250 kg");
    std::fs::write(&file, text).unwrap();
    let r = check_content(&root);
    assert!(r.mentions("changed after it was reviewed"), "{:?}", r.diagnostics);
}

#[test]
fn hand_flipped_status_is_an_error() {
    let root = copy_tree("good", "handflip");
    let file = root.join("math").join("q-ratios-percent.toml");
    let text = std::fs::read_to_string(&file).unwrap().replacen("status = \"draft\"", "status = \"reviewed\"", 1);
    std::fs::write(&file, text).unwrap();
    let r = check_content(&root);
    assert!(r.mentions("marked reviewed but it has no review"), "{:?}", r.diagnostics);
    assert!(approve(&root, "q-ratios-percent").is_err(), "approve refuses while there are errors");
}

#[test]
fn review_notes_are_validated() {
    let root = copy_tree("good", "badreviews");
    write_reviews(&root, "q-ratios-percent", &[("q-nope-0001", "question", "abc", "pass")]);
    write_reviews(&root, "not-a-skill", &[]);
    let r = check_content(&root);
    assert!(r.mentions("review refers to an unknown item"));
    assert!(r.mentions("not named after a skill"));
    let t = template(&root, "q-ratios-percent").unwrap();
    assert_eq!(t.matches("[[review]]").count(), 4, "3 questions + 1 lesson need review:\n{t}");
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

#[test]
fn week_path_days_are_checked() {
    let root = copy_tree("good", "days");
    let stage = |id: &str, day: u8| {
        format!("[[stage]]\nid = \"{id}\"\nday = {day}\ntitle = \"T\"\nprinciples = [\"Q.3.a\"]\nsize = 2\nmax_difficulty = 1\n\n")
    };
    std::fs::write(root.join("journey.toml"), stage("a", 2)).unwrap();
    assert!(check_content(&root).mentions("must start on day 1"));
    std::fs::write(root.join("journey.toml"), format!("{}{}", stage("a", 1), stage("b", 3).replace("Q.3.a", "Q.3.d"))).unwrap();
    assert!(check_content(&root).mentions("no gaps"));
    std::fs::write(root.join("journey.toml"), stage("a", 1).replace("size = 2", "size = 9")).unwrap();
    assert!(check_content(&root).mentions("questions exist for this stage"));
}

#[test]
fn study_links_are_checked_and_gated() {
    let root = copy_tree("good", "study");
    let good = "status = \"draft\"\n\n[[topic]]\nprinciple = \"Q.3.a\"\nsearch = \"unit rates\"\nlinks = [{ title = \"Rates\", url = \"https://example.org/rates\" }]\n";
    std::fs::write(root.join("study.toml"), good).unwrap();
    let r = check_content(&root);
    assert_eq!(r.errors(), 0, "{:?}", r.diagnostics);
    assert!(r.mentions("no study topic"), "missing principles are a warning");

    std::fs::write(root.join("study.toml"), good.replace("https://example.org/rates", "http://example.org/rates").replace("Q.3.a", "Q.9.z")).unwrap();
    let r = check_content(&root);
    assert!(r.mentions("plain https://"));
    assert!(r.mentions("unknown principle"));

    std::fs::write(root.join("study.toml"), good.replace("draft", "reviewed")).unwrap();
    assert!(check_content(&root).mentions("marked reviewed but it has no review"));
}

#[test]
fn demo_bank_builds_clean() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("demo-bank");
    ged_content::demo::build(&root.join("content"), &root.join("fixtures").join("demo-science"), &out).unwrap();
    let r = check_content(&out);
    for d in r.diagnostics.iter().filter(|d| d.level == ged_content::Level::Error) {
        eprintln!("{d}");
    }
    assert_eq!(r.errors(), 0);
    assert_eq!(r.stimuli.len(), 2);
    assert!(r.questions.iter().any(|(_, q)| q.kind == ged_content::QuestionType::Sort));
}
