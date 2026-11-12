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
    let h1 = question_hash(question(&r, "q-ratios-percent-0001"));
    let h2 = question_hash(question(&r, "q-ratios-percent-0002"));
    let h3 = question_hash(question(&r, "q-ratios-percent-0003"));
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

const GOOD_LESSON: &str = r#"
[[lesson]]
id = "q3a-unit-rate"
principle = "Q.3.a"
status = "draft"
title = "Unit rates"
goal = "You'll learn to find the cost of one."
how_to = ["Divide the total by how many.", "That is the cost for one."]
hook = "Per means for each one."

[lesson.problem]
text = "Three apples cost 6 dollars. How much is one?"
answer = "So one apple costs 2 dollars."

[lesson.idea]
text = '''
A unit rate tells you how much for one.

{{pictures}}
'''
figures = [{ kind = "array", rows = 1, cols = 3, label = "3 apples" }]

[[lesson.example]]
title = "Apples"
steps = [
  { say = "Split the cost over the three apples.", work = ['$6 \div 3 = 2$'] },
  { say = "That is the cost for one.", work = ["2 dollars each"] },
]
answer = "Each apple costs 2 dollars."
check = "Check: 3 times 2 is 6."

[lesson.example.turn]
prompt = "Eight pens cost 16 dollars. How much is one pen?"
answer = "2"
check = "16 / 8"
solution = "Divide 16 by 8 to get 2."

[[lesson.example]]
title = "Pears"
steps = [
  { say = "Split the cost over the four pears.", work = ['$12 \div 4 = 3$'] },
  { say = "That is the cost for one.", work = ["3 dollars each"] },
]
answer = "Each pear costs 3 dollars."
check = "Check: 4 times 3 is 12."

[lesson.example.turn]
prompt = "Five cans cost 10 dollars. How much is one can?"
choices = ["2", "5"]
answer = "2"
check = "10 / 5"
solution = "Divide 10 by 5 to get 2."

[[lesson.mistake]]
wrong = '$3 \div 6$'
right = '$6 \div 3$'
why = "Put the total first."

[[lesson.practice]]
prompt = "Two hats cost 8 dollars. How much is one hat?"
answer = "4"
check = "8 / 2"
solution = "Divide 8 by 2 to get 4."

[[lesson.practice]]
prompt = "Six eggs cost 3 dollars. How much is one egg, in dollars?"
answer = "0.5"
check = "3 / 6"
solution = "Divide 3 by 6 to get 0.5."

[lesson.review]
example = "6 dollars for 3 apples: 6 / 3 = 2 dollars each."
"#;

fn with_lesson(dest: &str, text: &str) -> PathBuf {
    let root = copy_tree("good", dest);
    std::fs::create_dir_all(root.join("lessons")).unwrap();
    std::fs::write(root.join("lessons").join("q-ratios-percent.toml"), text).unwrap();
    root
}

#[test]
fn a_well_formed_lesson_passes() {
    let r = check_content(&with_lesson("lesson-good", GOOD_LESSON));
    for d in &r.diagnostics {
        eprintln!("{d}");
    }
    assert_eq!(r.errors(), 0);
    assert_eq!(r.lessons.len(), 1);
}

#[test]
fn lesson_rules_are_enforced() {
    let bad = GOOD_LESSON
        .replace("title = \"Unit rates\"", "title = \"Unit rates, ratios\"")
        .replace("A unit rate tells you", "A unit rate simply tells you")
        .replace("check = \"16 / 8\"", "check = \"16 / 4\"")
        .replacen("  { say = \"That is the cost for one.\", work = [\"2 dollars each\"] },\n", "", 1);
    let r = check_content(&with_lesson("lesson-bad", &bad));
    assert!(r.mentions("title has a comma"), "{:?}", r.diagnostics);
    assert!(r.mentions("uses \"simply\""));
    assert!(r.mentions("WRONG KEY"));
    assert!(r.mentions("example 1 needs at least two steps"));
}

#[test]
fn a_lesson_needs_a_review_to_be_reviewed() {
    let root = with_lesson("lesson-gate", &GOOD_LESSON.replacen("status = \"draft\"", "status = \"reviewed\"", 1));
    let r = check_content(&root);
    assert!(r.mentions("marked reviewed but it has no review"), "{:?}", r.diagnostics);

    let root = with_lesson("lesson-approve", GOOD_LESSON);
    let r = check_content(&root);
    let h = ged_content::unit_hash(&r.lessons[0].1);
    write_reviews(&root, "q-ratios-percent", &[("q3a-unit-rate", "lesson", &h, "pass")]);
    let approved = approve(&root, "q-ratios-percent").expect("approve runs");
    assert!(approved.contains(&"q3a-unit-rate".to_string()), "{approved:?}");
    let r = check_content(&root);
    assert_eq!(r.lessons[0].1.status, Status::Reviewed);
}
