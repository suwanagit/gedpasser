//! Question bank schema, validator and answer checker.
//!
//! `check_content(root)` loads `root/subjects.toml`, `root/skills.toml`,
//! `root/principles.toml`, `root/stimuli/<subject>.toml` and every
//! `root/<subject>/<skill>.toml`, validates them, evaluates each question's
//! `check` expression against its stored answer, and returns a [`Report`].

pub mod expr;
pub mod extra;
pub mod review;
pub mod schema;
pub mod subjects;

pub use review::{approve, card_hash, lesson_hash, question_hash, stimulus_hash, strategy_hash, study_hash, template};
pub use subjects::{blank_markers, question_domain, stimulus_subject};

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

pub use schema::{
    Blank, Card, Domain, Figure, Formula, Group, Principle, PrincipleFile, Question, QuestionFile, QuestionType,
    Review, ReviewKind, Skill, SkillFile, Stage, Status, Stimulus, StrategyFile, StudyFile, StudyLink, StudyTopic,
    Subject, Table, Tactic, TestShape, Verdict,
};

/// Prompts whose word sets overlap at least this much are flagged.
pub const NEAR_DUPLICATE_THRESHOLD: f64 = 0.85;
/// Relative tolerance when comparing `check` with the stored answer.
pub const TOLERANCE: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub level: Level,
    pub file: PathBuf,
    pub id: Option<String>,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let level = match self.level {
            Level::Error => "error",
            Level::Warning => "warning",
        };
        write!(f, "{level}: {}", self.file.display())?;
        if let Some(id) = &self.id {
            write!(f, ":{id}")?;
        }
        write!(f, ": {}", self.message)
    }
}

#[derive(Debug, Default)]
pub struct Report {
    pub subjects: Vec<Subject>,
    pub skills: Vec<Skill>,
    pub stimuli: Vec<(PathBuf, Stimulus)>,
    pub principles: Vec<Principle>,
    pub questions: Vec<(PathBuf, Question)>,
    pub cards: Vec<(PathBuf, Card)>,
    pub journey: Vec<Stage>,
    pub strategy: Option<StrategyFile>,
    pub study: Option<StudyFile>,
    pub reviews: Vec<(PathBuf, Review)>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Report {
    pub fn errors(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.level == Level::Error).count()
    }
    pub fn warnings(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.level == Level::Warning).count()
    }
    /// True if any diagnostic contains `needle` (handy in tests).
    pub fn mentions(&self, needle: &str) -> bool {
        self.diagnostics.iter().any(|d| d.message.contains(needle))
    }

    /// A stimulus by id.
    pub fn stimulus(&self, id: &str) -> Option<&Stimulus> {
        self.stimuli.iter().map(|(_, s)| s).find(|s| s.id == id)
    }

    /// The stimulus a question points to, if any.
    pub fn stimulus_of(&self, q: &Question) -> Option<&Stimulus> {
        q.stimulus.as_deref().and_then(|id| self.stimulus(id))
    }

    pub(crate) fn push(&mut self, level: Level, file: &Path, id: Option<&str>, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic {
            level,
            file: file.to_path_buf(),
            id: id.map(str::to_owned),
            message: message.into(),
        });
    }
}

/// Load and validate only the skill map. Used by the server at startup.
pub fn load_skills(root: &Path) -> Result<Vec<Skill>, String> {
    let mut report = Report::default();
    subjects::check_subjects(root, &mut report);
    check_skills(root, &mut report);
    if report.errors() > 0 {
        let msgs: Vec<String> = report.diagnostics.iter().map(|d| d.to_string()).collect();
        return Err(msgs.join("\n"));
    }
    Ok(report.skills)
}

/// Run the validator and the answer checker over a content directory.
pub fn check_content(root: &Path) -> Report {
    let mut report = Report::default();
    subjects::check_subjects(root, &mut report);
    check_skills(root, &mut report);
    check_principles(root, &mut report);
    subjects::load_stimuli(root, &mut report);
    load_questions(root, &mut report);
    validate_questions(&mut report);
    check_answers(&mut report);
    check_duplicates(&mut report);
    extra::check_question_figures(&mut report);
    extra::check_cards(root, &mut report);
    extra::check_journey(root, &mut report);
    extra::check_strategy(root, &mut report);
    extra::check_study(root, &mut report);
    review::check_reviews(root, &mut report);
    report
}

pub(crate) fn read_toml<T: serde::de::DeserializeOwned>(path: &Path, report: &mut Report) -> Option<T> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            report.push(Level::Error, path, None, format!("cannot read file: {e}"));
            return None;
        }
    };
    match toml::from_str(&text) {
        Ok(v) => Some(v),
        Err(e) => {
            let msg = e.to_string().split_whitespace().collect::<Vec<_>>().join(" ");
            report.push(Level::Error, path, None, format!("invalid TOML: {msg}"));
            None
        }
    }
}

/// An official indicator code: capital letters, then one or two parts that
/// are a number or a lowercase letter. "Q.1.a", "SP.3.b", "R.2.1", "W.1".
fn is_target_code(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    (2..=3).contains(&parts.len())
        && (1..=3).contains(&parts[0].len())
        && parts[0].chars().all(|c| c.is_ascii_uppercase())
        && parts[1..].iter().all(|p| {
            !p.is_empty() && (p.chars().all(|c| c.is_ascii_digit()) || (p.len() == 1 && p.chars().all(|c| c.is_ascii_lowercase())))
        })
}

fn is_skill_id(s: &str) -> bool {
    s.len() > 2
        && s.starts_with(|c: char| c.is_ascii_lowercase())
        && s.contains('-')
        && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn check_skills(root: &Path, report: &mut Report) {
    let path = root.join("skills.toml");
    let Some(file) = read_toml::<SkillFile>(&path, report) else { return };
    let mut seen = HashSet::new();
    for s in &file.skill {
        let id = Some(s.id.as_str());
        if !is_skill_id(&s.id) {
            report.push(Level::Error, &path, id, "skill id must look like q-name or a-name");
        }
        match report.subjects.iter().find(|x| x.id == s.subject) {
            None => report.push(Level::Error, &path, id, format!("unknown subject {:?} (see subjects.toml)", s.subject)),
            Some(_) if s.subject == "math" => {
                let prefix_ok = match s.area.as_str() {
                    "quantitative" => s.id.starts_with("q-"),
                    "algebraic" => s.id.starts_with("a-"),
                    _ => {
                        report.push(Level::Error, &path, id, "area must be \"quantitative\" or \"algebraic\"");
                        true
                    }
                };
                if !prefix_ok {
                    report.push(Level::Error, &path, id, "id prefix does not match area (q- quantitative, a- algebraic)");
                }
            }
            Some(_) => {
                if !s.area.is_empty() {
                    report.push(Level::Error, &path, id, "area is for math skills only; other subjects use each question's topic");
                }
            }
        }
        if !seen.insert(s.id.clone()) {
            report.push(Level::Error, &path, id, "duplicate skill id");
        }
        if s.name.trim().is_empty() || s.description.trim().is_empty() {
            report.push(Level::Error, &path, id, "name and description must not be empty");
        }
        if !(1..=3).contains(&s.difficulty_min)
            || !(1..=3).contains(&s.difficulty_max)
            || s.difficulty_min > s.difficulty_max
        {
            report.push(Level::Error, &path, id, "difficulty range must be within 1-3 with min <= max");
        }
        if s.targets.is_empty() {
            report.push(Level::Error, &path, id, "skill must list at least one assessment target");
        }
        for t in &s.targets {
            if !is_target_code(t) {
                report.push(Level::Error, &path, id, format!("target {t:?} is not like \"Q.1.a\""));
            }
        }
    }
    report.skills = file.skill;
}

fn check_principles(root: &Path, report: &mut Report) {
    let path = root.join("principles.toml");
    let Some(file) = read_toml::<PrincipleFile>(&path, report) else { return };
    // Which skill lists each official target.
    let owner: HashMap<&str, &str> = report
        .skills
        .iter()
        .flat_map(|s| s.targets.iter().map(move |t| (t.as_str(), s.id.as_str())))
        .collect();
    let mut seen = HashSet::new();
    let mut diags = Vec::new();
    for p in &file.principle {
        let mut err = |m: String| diags.push((p.id.clone(), m));
        if !is_target_code(&p.id) {
            err("principle id must be an indicator code like \"Q.3.d\"".into());
        }
        if !seen.insert(p.id.clone()) {
            err("duplicate principle id".into());
        }
        match owner.get(p.id.as_str()) {
            None => err("principle is not listed in any skill's targets in skills.toml".into()),
            Some(&skill) if skill != p.skill => {
                err(format!("principle has skill {:?} but skills.toml lists it under {skill:?}", p.skill))
            }
            _ => {}
        }
        for (name, value) in [("name", &p.name), ("summary", &p.summary), ("lesson", &p.lesson)] {
            if value.trim().is_empty() {
                err(format!("{name} must not be empty"));
            }
        }
    }
    let mut missing: Vec<&&str> = owner.keys().filter(|t| !seen.contains(**t)).collect();
    missing.sort();
    for t in missing {
        diags.push((t.to_string(), "skills.toml lists this target but principles.toml has no principle for it".into()));
    }
    for (id, m) in diags {
        report.push(Level::Error, &path, Some(&id), m);
    }
    report.principles = file.principle;
}

fn load_questions(root: &Path, report: &mut Report) {
    let mut file_errors = Vec::new();
    let mut loaded = Vec::new();
    let mut sub_diags = Vec::new();
    for subject in &report.subjects {
        let skill_ids: HashSet<&str> =
            report.skills.iter().filter(|s| s.subject == subject.id).map(|s| s.id.as_str()).collect();
        let dir = root.join(&subject.id);
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            // A subject with no skills yet needs no question folder.
            Err(_) if skill_ids.is_empty() => continue,
            Err(e) => {
                file_errors.push((dir.clone(), format!("cannot read question directory: {e}")));
                continue;
            }
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect();
        paths.sort();
        for path in paths {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
            if !skill_ids.contains(stem.as_str()) {
                file_errors.push((path.clone(), format!("file name {stem:?} is not a skill id in skills.toml")));
            }
            let mut sub = Report::default();
            if let Some(file) = read_toml::<QuestionFile>(&path, &mut sub) {
                for q in file.question {
                    if q.skill != stem {
                        file_errors.push((
                            path.clone(),
                            format!("question {} has skill {:?} but lives in {stem}.toml", q.id, q.skill),
                        ));
                    }
                    loaded.push((path.clone(), q));
                }
            }
            sub_diags.extend(sub.diagnostics);
        }
    }
    report.diagnostics.extend(sub_diags);
    for (p, m) in file_errors {
        report.push(Level::Error, &p, None, m);
    }
    report.questions = loaded;
}

fn validate_questions(report: &mut Report) {
    let skills: HashMap<&str, &Skill> = report.skills.iter().map(|s| (s.id.as_str(), s)).collect();
    // stimulus id -> (subject, status)
    let stimuli: HashMap<&str, (String, Status)> =
        report.stimuli.iter().map(|(p, s)| (s.id.as_str(), (stimulus_subject(p), s.status))).collect();
    // principle id -> (skill, lesson status)
    let principles: HashMap<String, (String, Status)> =
        report.principles.iter().map(|p| (p.id.clone(), (p.skill.clone(), p.status))).collect();
    let mut seen_ids = HashSet::new();
    let mut diags = Vec::new();
    let mut warnings = Vec::new();
    for (path, q) in &report.questions {
        let mut err = |m: String| diags.push((Level::Error, path.clone(), q.id.clone(), m));

        // id = <skill>-NNNN
        let suffix = q.id.strip_prefix(&format!("{}-", q.skill));
        if !suffix.is_some_and(|s| s.len() == 4 && s.chars().all(|c| c.is_ascii_digit())) {
            err(format!("id must be \"{}-NNNN\" (four digits)", q.skill));
        }
        if !seen_ids.insert(q.id.clone()) {
            err("duplicate question id".into());
        }
        let skill = skills.get(q.skill.as_str()).copied();
        match skill {
            None => err(format!("unknown skill {:?}", q.skill)),
            Some(sk) => {
                let (lo, hi) = (sk.difficulty_min, sk.difficulty_max);
                if !(lo..=hi).contains(&q.difficulty) {
                    err(format!("difficulty {} is outside this skill's range {lo}-{hi}", q.difficulty));
                }
                // Every question counts toward one domain of its subject.
                if let Some(subject) = report.subjects.iter().find(|x| x.id == sk.subject) {
                    if sk.subject == "math" && q.topic.is_some() {
                        err("math questions take their domain from the skill's area; remove topic".into());
                    } else if sk.subject != "math" && q.topic.is_none() {
                        err("needs a topic (the official content-topic code, e.g. \"L.c.2\")".into());
                    } else {
                        let domain = question_domain(q, sk);
                        if !subject.domain.iter().any(|d| d.id == domain) {
                            err(format!("domain {domain:?} is not listed for subject {} in subjects.toml", subject.id));
                        }
                    }
                }
                let numeric = q.kind == QuestionType::FillIn && q.accept.is_empty() && parse_answer(&q.answer).is_some();
                if q.check.trim().is_empty() && (sk.subject == "math" || numeric) {
                    err("check must not be empty (required for math and for numeric answers)".into());
                }
                if let Some(id) = &q.stimulus {
                    match stimuli.get(id.as_str()) {
                        None => err(format!("unknown stimulus {id:?}")),
                        Some((subject, _)) if *subject != sk.subject => {
                            err(format!("stimulus {id} belongs to subject {subject}, not {}", sk.subject))
                        }
                        Some((_, status)) => {
                            if q.status == Status::Reviewed && *status != Status::Reviewed {
                                err(format!("reviewed question uses stimulus {id}, which is not reviewed yet"));
                            }
                        }
                    }
                }
            }
        }
        match principles.get(q.principle.as_str()) {
            None => err(format!("unknown principle {:?} (see principles.toml)", q.principle)),
            Some((skill, _)) if *skill != q.skill => err(format!(
                "principle {} belongs to skill {skill:?}, not {:?}",
                q.principle, q.skill
            )),
            Some((_, lesson_status)) => {
                if q.status == Status::Reviewed && *lesson_status != Status::Reviewed {
                    warnings.push((
                        path.clone(),
                        q.id.clone(),
                        format!("reviewed question links to principle {} whose lesson is not reviewed yet", q.principle),
                    ));
                }
            }
        }
        for (name, value) in [
            ("prompt", &q.prompt),
            ("explanation", &q.explanation),
            ("common_mistake", &q.common_mistake),
        ] {
            if value.trim().is_empty() {
                err(format!("{name} must not be empty"));
            }
        }
        for m in subjects::format_problems(q) {
            err(m);
        }
    }
    for (level, path, id, m) in diags {
        report.push(level, &path, Some(&id), m);
    }
    for (path, id, m) in warnings {
        report.push(Level::Warning, &path, Some(&id), m);
    }
}

/// Parse an answer string as a number. Accepts integers, decimals,
/// thousands separators ("1,200"), a leading "$", fractions ("3/4"), mixed
/// numbers ("1 1/2", "-2 3/4") and percents ("25%" parses as 25).
/// Math markup is understood too: "$\frac{3}{4}$", "$1\dfrac{1}{2}$",
/// "$25\%$".
pub fn parse_answer(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.contains('\\') || (s.len() >= 2 && s.starts_with('$') && s.ends_with('$') && s[1..].contains('$')) {
        return parse_answer(&delatex(s)?);
    }
    parse_plain(s)
}

/// Turn simple LaTeX number markup into plain text: strips `$…$`, turns
/// `\frac{a}{b}` / `\dfrac{a}{b}` into ` a/b`, and `\%` into `%`.
fn delatex(s: &str) -> Option<String> {
    let inner = s.strip_prefix('$').and_then(|t| t.strip_suffix('$')).unwrap_or(s);
    let mut t = inner.replace("\\dfrac", "\\frac").replace("\\tfrac", "\\frac").replace("\\%", "%");
    t = t.replace("\\,", "").replace("{,}", ",").replace("\\$", "");
    while let Some(pos) = t.find("\\frac{") {
        let rest = &t[pos + 6..];
        let n_end = rest.find('}')?;
        let num = &rest[..n_end];
        let rest2 = rest[n_end + 1..].strip_prefix('{')?;
        let d_end = rest2.find('}')?;
        let den = &rest2[..d_end];
        let tail = &rest2[d_end + 1..];
        t = format!("{} {num}/{den}{tail}", t[..pos].trim_end());
    }
    if t.contains('\\') || t.contains('$') {
        return None;
    }
    Some(t.trim().to_string())
}

fn parse_plain(s: &str) -> Option<f64> {
    let mut t = s.trim();
    let mut sign = 1.0;
    if let Some(rest) = t.strip_prefix('-') {
        sign = -1.0;
        t = rest.trim_start();
    }
    t = t.strip_prefix('$').unwrap_or(t);
    t = t.strip_suffix('%').unwrap_or(t).trim_end();
    let t = t.replace(',', "");
    let parts: Vec<&str> = t.split_whitespace().collect();
    let plain = |p: &str| -> Option<f64> {
        if p.is_empty() || !p.chars().all(|c| c.is_ascii_digit() || c == '.') {
            return None;
        }
        p.parse::<f64>().ok()
    };
    let fraction = |p: &str| -> Option<f64> {
        let (n, d) = p.split_once('/')?;
        let d = plain(d)?;
        if d == 0.0 {
            return None;
        }
        Some(plain(n)? / d)
    };
    let value = match parts.as_slice() {
        [one] => plain(one).or_else(|| fraction(one))?,
        [whole, frac] => plain(whole)?.trunc() + fraction(frac)?,
        _ => return None,
    };
    Some(sign * value)
}

/// True when two numbers agree within [`TOLERANCE`] (relative, floor of 1).
pub fn close_enough(a: f64, b: f64) -> bool {
    (a - b).abs() <= TOLERANCE * a.abs().max(b.abs()).max(1.0)
}

fn check_answers(report: &mut Report) {
    let mut diags = Vec::new();
    for (path, q) in &report.questions {
        if q.check.trim().is_empty() {
            continue; // already reported by the validator
        }
        let computed = match expr::eval(&q.check) {
            Ok(v) => v,
            Err(e) => {
                diags.push((path.clone(), q.id.clone(), format!("check expression {:?} failed: {e}", q.check)));
                continue;
            }
        };
        let expected = match q.answer_value.or_else(|| parse_answer(&q.answer)) {
            Some(v) => v,
            None => {
                diags.push((
                    path.clone(),
                    q.id.clone(),
                    format!("answer {:?} is not a number; add answer_value", q.answer),
                ));
                continue;
            }
        };
        if !close_enough(computed, expected) {
            diags.push((
                path.clone(),
                q.id.clone(),
                format!("WRONG KEY: check {:?} = {computed} but answer is {expected}", q.check),
            ));
        }
    }
    for (path, id, m) in diags {
        report.push(Level::Error, &path, Some(&id), m);
    }
}

fn words(s: &str) -> BTreeSet<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn check_duplicates(report: &mut Report) {
    let sets: Vec<BTreeSet<String>> = report.questions.iter().map(|(_, q)| words(&q.prompt)).collect();
    // Questions on one stimulus may share a short stem ("Which conclusion…").
    let same_set = |a: &Question, b: &Question| a.stimulus.is_some() && a.stimulus != b.stimulus;
    let mut diags = Vec::new();
    for i in 0..sets.len() {
        for j in (i + 1)..sets.len() {
            let (pa, a) = &report.questions[i];
            let (_, b) = &report.questions[j];
            if same_set(a, b) || same_set(b, a) {
                continue;
            }
            if a.prompt.trim() == b.prompt.trim() {
                diags.push((Level::Error, pa.clone(), a.id.clone(), format!("same prompt as {}", b.id)));
                continue;
            }
            let inter = sets[i].intersection(&sets[j]).count() as f64;
            let union = sets[i].union(&sets[j]).count() as f64;
            if union > 0.0 && inter / union >= NEAR_DUPLICATE_THRESHOLD {
                diags.push((
                    Level::Warning,
                    pa.clone(),
                    a.id.clone(),
                    format!("prompt is nearly the same as {} ({:.0}% word overlap)", b.id, 100.0 * inter / union),
                ));
            }
        }
    }
    for (level, path, id, m) in diags {
        report.push(level, &path, Some(&id), m);
    }
}

#[cfg(test)]
mod tests {
    use super::parse_answer;

    #[test]
    fn answers() {
        assert_eq!(parse_answer("360"), Some(360.0));
        assert_eq!(parse_answer("1,200"), Some(1200.0));
        assert_eq!(parse_answer("$12.50"), Some(12.5));
        assert_eq!(parse_answer("3/4"), Some(0.75));
        assert_eq!(parse_answer("1 1/2"), Some(1.5));
        assert_eq!(parse_answer("-2 3/4"), Some(-2.75));
        assert_eq!(parse_answer("25%"), Some(25.0));
        assert_eq!(parse_answer("-0.5"), Some(-0.5));
        assert_eq!(parse_answer("2x + 3"), None);
        assert_eq!(parse_answer("1/0"), None);
    }

    #[test]
    fn latex_answers() {
        assert_eq!(parse_answer(r"$\frac{3}{4}$"), Some(0.75));
        assert_eq!(parse_answer(r"$-\frac{1}{2}$"), Some(-0.5));
        assert_eq!(parse_answer(r"$1\dfrac{1}{2}$"), Some(1.5));
        assert_eq!(parse_answer(r"$25\%$"), Some(25.0));
        assert_eq!(parse_answer("$12$"), Some(12.0));
        assert_eq!(parse_answer("$12.50"), Some(12.5)); // dollars, not math
        assert_eq!(parse_answer(r"$x^{2}$"), None);
        assert_eq!(parse_answer(r"$\sqrt{2}$"), None);
    }
}
