//! Full lessons (`content/lessons/<skill>.toml`), checked against the rules
//! in `docs/LESSON_STYLE.md` that code can check.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::extra::prose_only;
use crate::schema::{Lesson, LessonFile};
use crate::{Level, Report, close_enough, expr, parse_answer, read_toml};

/// Words the style guide bans.
pub const BANNED_WORDS: [&str; 3] = ["simply", "obviously", "just"];
/// Sentences longer than this are flagged.
pub const MAX_SENTENCE_WORDS: usize = 20;
/// Reading grade (Flesch–Kincaid) above which a lesson is flagged.
pub const MAX_GRADE: f64 = 7.0;

/// The id prefix a lesson for `principle` must start with: "Q.1.b" -> "q1b-".
pub fn id_prefix(principle: &str) -> String {
    format!("{}-", principle.replace('.', "").to_lowercase())
}

/// Every piece of prose in a lesson, labeled, in reading order.
pub fn texts(l: &Lesson) -> Vec<(String, &str)> {
    let mut out: Vec<(String, &str)> = vec![("title".into(), l.title.as_str()), ("why".into(), &l.why), ("idea".into(), &l.idea)];
    for (i, e) in l.example.iter().enumerate() {
        out.push((format!("example {} title", i + 1), &e.title));
        for (j, s) in e.steps.iter().enumerate() {
            out.push((format!("example {} step {}", i + 1, j + 1), &s.work));
            out.push((format!("example {} step {} why", i + 1, j + 1), &s.why));
        }
        out.push((format!("example {} answer", i + 1), &e.answer));
    }
    out.push(("rule".into(), &l.rule.text));
    for t in &l.rule.terms {
        out.push((format!("term {}", t.term), &t.means));
    }
    out.push(("mistake wrong".into(), &l.mistake.wrong));
    out.push(("mistake right".into(), &l.mistake.right));
    out.push(("mistake why".into(), &l.mistake.why));
    for (i, t) in l.tries.iter().enumerate() {
        out.push((format!("try {} prompt", i + 1), &t.prompt));
        out.push((format!("try {} solution", i + 1), &t.solution));
    }
    for (i, r) in l.recap.iter().enumerate() {
        out.push((format!("recap {}", i + 1), r));
    }
    out
}

fn words_of(s: &str) -> Vec<String> {
    prose_only(s)
        .split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '’'))
        .filter(|w| w.chars().any(char::is_alphabetic))
        .map(|w| w.to_lowercase())
        .collect()
}

/// Sentences of prose (math removed), split at . ! ? and line breaks.
pub fn sentences(s: &str) -> Vec<String> {
    prose_only(s)
        .split(|c: char| c == '.' || c == '!' || c == '?' || c == '\n')
        .map(str::trim)
        .filter(|x| x.chars().any(char::is_alphabetic))
        .map(str::to_string)
        .collect()
}

/// A rough syllable count: groups of vowels, minus a silent final "e".
pub fn syllables(word: &str) -> usize {
    let w: Vec<char> = word.to_lowercase().chars().filter(|c| c.is_ascii_alphabetic()).collect();
    if w.is_empty() {
        return 0;
    }
    let vowel = |c: char| "aeiouy".contains(c);
    let mut count = 0;
    let mut prev = false;
    for &c in &w {
        let v = vowel(c);
        if v && !prev {
            count += 1;
        }
        prev = v;
    }
    if w.len() > 2 && w[w.len() - 1] == 'e' && !vowel(w[w.len() - 2]) && w[w.len() - 2] != 'l' {
        count -= 1;
    }
    count.max(1)
}

/// Flesch–Kincaid grade level of a lesson's explanation text: why, idea,
/// the reasons in the examples, the rule and the mistake.
pub fn reading_grade(l: &Lesson) -> f64 {
    let mut parts: Vec<&str> = vec![&l.why, &l.idea, &l.rule.text, &l.mistake.why];
    for e in &l.example {
        parts.extend(e.steps.iter().map(|s| s.why.as_str()));
    }
    let (mut n_sent, mut n_words, mut n_syl) = (0usize, 0usize, 0usize);
    for p in parts {
        for sent in sentences(p) {
            let w = words_of(&sent);
            if w.is_empty() {
                continue;
            }
            n_sent += 1;
            n_words += w.len();
            n_syl += w.iter().map(|x| syllables(x)).sum::<usize>();
        }
    }
    if n_sent == 0 || n_words == 0 {
        return 0.0;
    }
    0.39 * (n_words as f64 / n_sent as f64) + 11.8 * (n_syl as f64 / n_words as f64) - 15.59
}

pub(crate) fn check_lessons(root: &Path, report: &mut Report) {
    let dir = root.join("lessons");
    let mut paths: Vec<PathBuf> = match std::fs::read_dir(&dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect(),
        Err(_) => Vec::new(), // no lessons yet
    };
    paths.sort();
    let principle_skill: HashMap<&str, &str> =
        report.principles.iter().map(|p| (p.id.as_str(), p.skill.as_str())).collect();
    let skill_ids: HashSet<&str> = report.skills.iter().map(|s| s.id.as_str()).collect();
    let mut seen = HashSet::new();
    let mut loaded: Vec<(PathBuf, Lesson)> = Vec::new();
    let mut diags: Vec<(Level, PathBuf, Option<String>, String)> = Vec::new();
    for path in paths {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
        if !skill_ids.contains(stem.as_str()) {
            diags.push((Level::Error, path.clone(), None, format!("file name {stem:?} is not a skill id in skills.toml")));
        }
        let mut sub = Report::default();
        if let Some(file) = read_toml::<LessonFile>(&path, &mut sub) {
            for l in file.lesson {
                let mut out = Vec::new();
                lesson_problems(&l, &stem, &principle_skill, &mut out);
                if !seen.insert(l.id.clone()) {
                    out.push((Level::Error, "duplicate lesson id".to_string()));
                }
                for (level, m) in out {
                    diags.push((level, path.clone(), Some(l.id.clone()), m));
                }
                loaded.push((path.clone(), l));
            }
        }
        report.diagnostics.extend(sub.diagnostics);
    }
    for (level, path, id, m) in diags {
        report.push(level, &path, id.as_deref(), m);
    }
    report.lessons = loaded;
}

fn lesson_problems(l: &Lesson, skill: &str, principle_skill: &HashMap<&str, &str>, out: &mut Vec<(Level, String)>) {
    let mut err = |m: String| out.push((Level::Error, m));
    match principle_skill.get(l.principle.as_str()) {
        None => err(format!("unknown principle {:?}", l.principle)),
        Some(&s) if s != skill => err(format!("principle {} belongs to skill {s}, but the lesson is in {skill}.toml", l.principle)),
        _ => {}
    }
    let prefix = id_prefix(&l.principle);
    let rest = l.id.strip_prefix(&prefix).unwrap_or("");
    if rest.is_empty() || !rest.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        err(format!("id must look like \"{prefix}short-name\""));
    }
    if l.title.contains(',') {
        err("title has a comma: one idea per lesson, so split it".into());
    }
    if l.example.len() != 2 {
        err(format!("needs exactly two worked examples (has {})", l.example.len()));
    }
    for (i, e) in l.example.iter().enumerate() {
        if e.steps.len() < 2 {
            err(format!("example {} needs at least two steps", i + 1));
        }
        if e.title.trim().is_empty() || e.answer.trim().is_empty() {
            err(format!("example {} needs a title and an answer", i + 1));
        }
        for (j, s) in e.steps.iter().enumerate() {
            if s.work.trim().is_empty() || s.why.trim().is_empty() {
                err(format!("example {} step {} needs both `do` and `why`", i + 1, j + 1));
            }
        }
    }
    for t in &l.rule.terms {
        if t.term.trim().is_empty() || t.means.trim().is_empty() {
            err("every rule term needs a term and what it means".into());
        }
    }
    if !(2..=3).contains(&l.tries.len()) {
        err(format!("needs 2 or 3 try-it questions (has {})", l.tries.len()));
    }
    for (i, t) in l.tries.iter().enumerate() {
        let n = i + 1;
        if t.prompt.trim().is_empty() || t.answer.trim().is_empty() || t.solution.trim().is_empty() {
            err(format!("try {n} needs a prompt, an answer and a solution"));
        }
        if !t.choices.is_empty() {
            if t.choices.len() < 2 {
                err(format!("try {n} needs at least two choices"));
            }
            if !t.choices.iter().any(|c| c.trim() == t.answer.trim()) {
                err(format!("try {n}: answer {:?} is not one of the choices", t.answer));
            }
        }
        let number = parse_answer(&t.answer);
        if number.is_some() && t.check.trim().is_empty() {
            err(format!("try {n}: a numeric answer needs a check expression"));
        }
        if !t.check.trim().is_empty() {
            match (expr::eval(&t.check), number) {
                (Err(e), _) => err(format!("try {n}: check {:?} failed: {e}", t.check)),
                (Ok(_), None) => err(format!("try {n}: answer {:?} is not a number but has a check", t.answer)),
                (Ok(v), Some(want)) if !close_enough(v, want) => {
                    err(format!("try {n}: WRONG KEY: check {:?} = {v} but answer is {want}", t.check))
                }
                _ => {}
            }
        }
    }
    if !(2..=4).contains(&l.recap.len()) {
        err(format!("recap needs 2 to 4 lines (has {})", l.recap.len()));
    }
    for (name, text) in texts(l) {
        if text.trim().is_empty() && !name.starts_with("term") {
            err(format!("{name} must not be empty"));
        }
        let words = words_of(text);
        for banned in BANNED_WORDS {
            if words.iter().any(|w| w == banned) {
                err(format!("{name} uses \"{banned}\" (banned by the style guide)"));
            }
        }
        if !text.replace("\\$", "").matches('$').count().is_multiple_of(2) {
            err(format!("{name} has an unmatched $ (write a dollar sign as \\$)"));
        }
    }
    let mut warn = |m: String| out.push((Level::Warning, m));
    if l.title.to_lowercase().split_whitespace().any(|w| w == "and") {
        warn("title has \"and\": is this two ideas?".into());
    }
    for (name, text) in texts(l) {
        for sent in sentences(text) {
            let n = words_of(&sent).len();
            if n > MAX_SENTENCE_WORDS {
                warn(format!("{name}: a sentence has {n} words (keep to {MAX_SENTENCE_WORDS}): {sent:?}"));
            }
        }
    }
    let grade = reading_grade(l);
    if grade > MAX_GRADE {
        warn(format!("reads at about grade {grade:.1}; aim for 6 (shorter sentences, shorter words)"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syllable_counts() {
        assert_eq!(syllables("cat"), 1);
        assert_eq!(syllables("factor"), 2);
        assert_eq!(syllables("divide"), 2);
        assert_eq!(syllables("biggest"), 2);
        assert_eq!(syllables("table"), 2);
    }

    #[test]
    fn prefixes() {
        assert_eq!(id_prefix("Q.1.b"), "q1b-");
        assert_eq!(id_prefix("A.2.d"), "a2d-");
    }

    #[test]
    fn sentences_skip_math() {
        let s = sentences("You split it. Then $12 \\div 3 = 4$. Done!");
        assert_eq!(s.len(), 3);
    }
}
