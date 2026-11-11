//! Validation for figures, cards (short lessons), the journey and the
//! strategy file.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::schema::{Card, CardFile, Figure, JourneyFile, StrategyFile};
use crate::{Level, Report, read_toml};

/// Cards must stay short: words outside of math, across all text fields.
pub const CARD_MAX_WORDS: usize = 90;
pub const CARD_MAX_STEPS: usize = 4;

/// Problems with a figure, as messages (empty when fine).
pub fn figure_problems(f: &Figure) -> Vec<String> {
    let mut out = Vec::new();
    match f {
        Figure::Plane { x_min, x_max, y_min, y_max, curves, .. } => {
            if x_min >= x_max || y_min >= y_max {
                out.push("plane needs x_min < x_max and y_min < y_max".into());
            }
            for c in curves {
                let mut finite = 0;
                let steps = 40;
                for i in 0..=steps {
                    let x = x_min + (x_max - x_min) * i as f64 / steps as f64;
                    match crate::expr::eval_x(c, x) {
                        Ok(y) if y.is_finite() => finite += 1,
                        Ok(_) => {}
                        Err(e) if !e.contains("division by zero") && !e.contains("negative") => {
                            out.push(format!("curve {c:?} does not evaluate: {e}"));
                            break;
                        }
                        Err(_) => {}
                    }
                }
                if finite == 0 {
                    out.push(format!("curve {c:?} has no points in the window"));
                }
            }
        }
        Figure::NumberLine { min, max, points, ray } => {
            if min >= max {
                out.push("number line needs min < max".into());
            }
            for p in points {
                if p.x < *min || p.x > *max {
                    out.push(format!("number line point {} is outside {min}..{max}", p.x));
                }
            }
            if let Some(r) = ray {
                if r.from < *min || r.from > *max {
                    out.push("number line ray starts outside the line".into());
                }
            }
        }
        Figure::Bars { bars, .. } => {
            if bars.is_empty() {
                out.push("bar graph needs bars".into());
            }
            if bars.iter().any(|b| b.value < 0.0) {
                out.push("bar values must not be negative".into());
            }
        }
        Figure::Dots { counts, .. } => {
            if counts.is_empty() {
                out.push("dot plot needs counts".into());
            }
        }
        Figure::Box { min, q1, median, q3, max, axis_min, axis_max, .. } => {
            if !(min <= q1 && q1 <= median && median <= q3 && q3 <= max) {
                out.push("box plot needs min ≤ q1 ≤ median ≤ q3 ≤ max".into());
            }
            if axis_min > min || axis_max < max {
                out.push("box plot axis must cover min..max".into());
            }
        }
        Figure::Shape { polygons, circles, segments, labels } => {
            if polygons.is_empty() && circles.is_empty() && segments.is_empty() && labels.is_empty() {
                out.push("shape figure is empty".into());
            }
            for p in polygons {
                if p.points.len() < 3 {
                    out.push("a polygon needs at least 3 points".into());
                }
                if p.side_labels.len() > p.points.len() {
                    out.push("more side labels than sides".into());
                }
                if p.right_angles.iter().any(|&i| i >= p.points.len()) {
                    out.push("right-angle mark on a vertex that doesn't exist".into());
                }
            }
            if circles.iter().any(|c| c.r <= 0.0) {
                out.push("circle radius must be positive".into());
            }
        }
    }
    out
}

/// Remove `$...$` / `$$...$$` math (and `\$`) so only prose words remain.
pub fn prose_only(s: &str) -> String {
    let mut out = String::new();
    let mut in_math = false;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && chars.get(i + 1) == Some(&'$') {
            if !in_math {
                out.push('$');
            }
            i += 2;
            continue;
        }
        if c == '$' {
            if chars.get(i + 1) == Some(&'$') {
                i += 1;
            }
            if !in_math {
                out.push_str(" M ");
            }
            in_math = !in_math;
            i += 1;
            continue;
        }
        if !in_math {
            out.push(c);
        }
        i += 1;
    }
    out
}

pub fn word_count(s: &str) -> usize {
    prose_only(s).split_whitespace().filter(|w| w.chars().any(char::is_alphanumeric)).count()
}

fn card_words(c: &Card) -> usize {
    word_count(&c.key) + c.steps.iter().map(|s| word_count(s)).sum::<usize>() + word_count(&c.example) + word_count(&c.hack)
}

fn balanced_dollars(s: &str) -> bool {
    let unescaped = s.replace("\\$", "");
    unescaped.matches('$').count() % 2 == 0
}

pub(crate) fn check_cards(root: &Path, report: &mut Report) {
    let dir = root.join("cards");
    let mut paths: Vec<PathBuf> = match std::fs::read_dir(&dir) {
        Ok(e) => e
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect(),
        Err(_) => Vec::new(),
    };
    paths.sort();
    let principle_skill: HashMap<&str, &str> =
        report.principles.iter().map(|p| (p.id.as_str(), p.skill.as_str())).collect();
    let mut diags = Vec::new();
    let mut seen = HashSet::new();
    let mut loaded = Vec::new();
    for path in paths {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
        let mut sub = Report::default();
        let Some(file) = read_toml::<CardFile>(&path, &mut sub) else {
            report.diagnostics.extend(sub.diagnostics);
            continue;
        };
        for c in file.card {
            let mut err = |m: String| diags.push((Level::Error, path.clone(), Some(c.id.clone()), m));
            match principle_skill.get(c.id.as_str()) {
                None => err("card id is not a principle in principles.toml".into()),
                Some(&skill) if skill != stem => err(format!("card for a {skill} principle is in {stem}.toml")),
                _ => {}
            }
            if !seen.insert(c.id.clone()) {
                err("duplicate card".into());
            }
            let tw = c.title.split_whitespace().count();
            if !(1..=5).contains(&tw) {
                err(format!("title should be 1–5 words (has {tw})"));
            }
            for (name, v) in [("key", &c.key), ("example", &c.example), ("hack", &c.hack)] {
                if v.trim().is_empty() {
                    err(format!("{name} must not be empty"));
                }
                if !balanced_dollars(v) {
                    err(format!("{name} has an unmatched $ (write a dollar sign as \\$)"));
                }
            }
            if c.steps.is_empty() || c.steps.len() > CARD_MAX_STEPS {
                err(format!("steps: 1 to {CARD_MAX_STEPS} (has {})", c.steps.len()));
            }
            for s in &c.steps {
                if !balanced_dollars(s) {
                    err("a step has an unmatched $".into());
                }
                if word_count(s) > 16 {
                    err(format!("step too long ({} words, max 16): {s:?}", word_count(s)));
                }
            }
            let words = card_words(&c);
            if words > CARD_MAX_WORDS {
                err(format!("card is {words} words; keep it to {CARD_MAX_WORDS} or fewer (math doesn't count)"));
            }
            if let Some(f) = &c.figure {
                for p in figure_problems(f) {
                    err(format!("figure: {p}"));
                }
            }
            loaded.push((path.clone(), c));
        }
        report.diagnostics.extend(sub.diagnostics);
    }
    let mut missing: Vec<&str> =
        report.principles.iter().map(|p| p.id.as_str()).filter(|id| !seen.contains(*id)).collect();
    missing.sort();
    if !missing.is_empty() && !report.principles.is_empty() && dir.exists() {
        diags.push((
            Level::Warning,
            dir.clone(),
            None,
            format!("{} principle(s) have no card yet: {}", missing.len(), missing.join(", ")),
        ));
    }
    for (level, path, id, m) in diags {
        report.diagnostics.push(crate::Diagnostic { level, file: path, id, message: m });
    }
    report.cards = loaded;
}

pub(crate) fn check_question_figures(report: &mut Report) {
    let mut diags = Vec::new();
    for (path, q) in &report.questions {
        if let Some(f) = &q.figure {
            for p in figure_problems(f) {
                diags.push((path.clone(), q.id.clone(), format!("figure: {p}")));
            }
        }
    }
    for (path, id, m) in diags {
        report.diagnostics.push(crate::Diagnostic { level: Level::Error, file: path, id: Some(id), message: m });
    }
}

pub(crate) fn check_journey(root: &Path, report: &mut Report) {
    let path = root.join("journey.toml");
    if !path.exists() {
        return;
    }
    let Some(file) = read_toml::<JourneyFile>(&path, report) else { return };
    let known: HashSet<&str> = report.principles.iter().map(|p| p.id.as_str()).collect();
    let mut ids = HashSet::new();
    let mut taught: HashMap<String, String> = HashMap::new();
    let mut diags = Vec::new();
    for (i, s) in file.stage.iter().enumerate() {
        let mut err = |m: String| diags.push((Level::Error, Some(s.id.clone()), m));
        if !ids.insert(s.id.clone()) {
            err("duplicate stage id".into());
        }
        if s.title.trim().is_empty() {
            err("title must not be empty".into());
        }
        if !(2..=12).contains(&s.size) {
            err("size must be 2–12".into());
        }
        if !(1..=3).contains(&s.max_difficulty) {
            err("max_difficulty must be 1–3".into());
        }
        if s.checkpoint {
            if !s.principles.is_empty() {
                err("a checkpoint takes no principles (it mixes the stages before it)".into());
            }
            if i == 0 {
                err("the journey can't start with a checkpoint".into());
            }
        } else if s.principles.is_empty() {
            err("a stage needs at least one principle".into());
        }
        for p in &s.principles {
            if !known.contains(p.as_str()) {
                err(format!("unknown principle {p}"));
            } else if let Some(other) = taught.insert(p.clone(), s.id.clone()) {
                err(format!("principle {p} is already taught in {other}"));
            }
        }
    }
    let mut missing: Vec<&str> = known.iter().copied().filter(|p| !taught.contains_key(*p)).collect();
    missing.sort();
    if !missing.is_empty() {
        diags.push((Level::Warning, None, format!("principles not in the journey: {}", missing.join(", "))));
    }
    for (level, id, m) in diags {
        report.diagnostics.push(crate::Diagnostic { level, file: path.clone(), id, message: m });
    }
    report.journey = file.stage;
}

pub(crate) fn check_strategy(root: &Path, report: &mut Report) {
    let path = root.join("strategy.toml");
    if !path.exists() {
        return;
    }
    let Some(file) = read_toml::<StrategyFile>(&path, report) else { return };
    let known: HashSet<&str> = report.principles.iter().map(|p| p.id.as_str()).collect();
    let mut seen = HashSet::new();
    let mut errs = Vec::new();
    for p in file.high_yield.iter().chain(file.medium_yield.iter()) {
        if !known.contains(p.as_str()) {
            errs.push(format!("unknown principle {p} in yield lists"));
        }
        if !seen.insert(p) {
            errs.push(format!("principle {p} is listed twice in yield lists"));
        }
    }
    let mut tactic_ids = HashSet::new();
    for t in &file.tactic {
        if !tactic_ids.insert(&t.id) {
            errs.push(format!("duplicate tactic id {}", t.id));
        }
        if t.title.trim().is_empty() || t.text.trim().is_empty() {
            errs.push(format!("tactic {} needs a title and text", t.id));
        }
        if word_count(&t.text) > 40 {
            errs.push(format!("tactic {} is {} words; keep it to 40", t.id, word_count(&t.text)));
        }
    }
    for f in &file.formula {
        if f.tex.trim().is_empty() || f.name.trim().is_empty() {
            errs.push(format!("formula {:?} needs a name and tex", f.name));
        }
    }
    for m in errs {
        report.diagnostics.push(crate::Diagnostic { level: Level::Error, file: path.clone(), id: None, message: m });
    }
    report.strategy = Some(file);
}
