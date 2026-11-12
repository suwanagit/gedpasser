//! The question bank in memory, plus its sync into SQLite at startup.
//! The TOML files stay the source of truth; the database only holds ids,
//! metadata and content hashes so attempts can reference questions.

use std::collections::HashMap;
use std::path::Path;

use ged_content::{
    Card, Principle, Question, QuestionType, Skill, Stage, Status, Stimulus, StrategyFile, StudyFile, Subject, check_content,
};
use sqlx::SqlitePool;

pub struct Bank {
    /// The GED tests, Math first (`subjects.toml`).
    pub subjects: Vec<Subject>,
    pub skills: Vec<Skill>,
    /// Shared texts, tables and figures, by id.
    pub stimuli: HashMap<String, Stimulus>,
    /// In skill order, then file order within each skill.
    pub principles: Vec<Principle>,
    pub questions: HashMap<String, Question>,
    /// Question ids per skill, in file order.
    pub by_skill: HashMap<String, Vec<String>>,
    pub hashes: HashMap<String, String>,
    /// Short lesson cards by principle id.
    pub cards: HashMap<String, Card>,
    /// The guided path, in order.
    pub journey: Vec<Stage>,
    pub strategy: Option<StrategyFile>,
    pub study: Option<StudyFile>,
}

impl Bank {
    /// Load and fully check the content folder. Any validator or answer
    /// checker error is fatal, so a broken bank is never served.
    pub fn load(root: &Path) -> Result<Bank, String> {
        let report = check_content(root);
        if report.errors() > 0 {
            let lines: Vec<String> = report
                .diagnostics
                .iter()
                .filter(|d| d.level == ged_content::Level::Error)
                .map(|d| d.to_string())
                .collect();
            return Err(format!(
                "the question bank has {} error(s); run `cargo content`:\n{}",
                lines.len(),
                lines.join("\n")
            ));
        }
        let mut questions = HashMap::new();
        let mut by_skill: HashMap<String, Vec<String>> = HashMap::new();
        let mut hashes = HashMap::new();
        for (_, q) in &report.questions {
            by_skill.entry(q.skill.clone()).or_default().push(q.id.clone());
            hashes.insert(q.id.clone(), ged_content::question_hash(q, report.stimulus_of(q)));
        }
        for (_, q) in report.questions {
            questions.insert(q.id.clone(), q);
        }
        let stimuli = report.stimuli.into_iter().map(|(_, s)| (s.id.clone(), s)).collect();
        let skill_order: HashMap<&str, usize> =
            report.skills.iter().enumerate().map(|(i, s)| (s.id.as_str(), i)).collect();
        let mut principles = report.principles.clone();
        principles.sort_by_key(|p| skill_order.get(p.skill.as_str()).copied().unwrap_or(usize::MAX));
        let cards = report.cards.into_iter().map(|(_, c)| (c.id.clone(), c)).collect();
        Ok(Bank {
            subjects: report.subjects,
            skills: report.skills,
            stimuli,
            principles,
            questions,
            by_skill,
            hashes,
            cards,
            journey: report.journey,
            strategy: report.strategy,
            study: report.study,
        })
    }

    /// A principle's card, if learners may see it.
    pub fn card(&self, principle: &str, serve_drafts: bool) -> Option<&Card> {
        self.cards.get(principle).filter(|c| status_servable(c.status, serve_drafts))
    }

    /// The strategy file, if learners may see it.
    pub fn strategy(&self, serve_drafts: bool) -> Option<&StrategyFile> {
        self.strategy.as_ref().filter(|s| status_servable(s.status, serve_drafts))
    }

    /// The study links file, if learners may see it.
    pub fn study(&self, serve_drafts: bool) -> Option<&StudyFile> {
        self.study.as_ref().filter(|s| status_servable(s.status, serve_drafts))
    }

    pub fn principle(&self, id: &str) -> Option<&Principle> {
        self.principles.iter().find(|p| p.id == id)
    }

    pub fn skill(&self, id: &str) -> Option<&Skill> {
        self.skills.iter().find(|s| s.id == id)
    }

    pub fn subject(&self, id: &str) -> Option<&Subject> {
        self.subjects.iter().find(|s| s.id == id)
    }

    /// The subject a skill belongs to ("math" if unknown).
    pub fn skill_subject(&self, skill: &str) -> &str {
        self.skill(skill).map(|s| s.subject.as_str()).unwrap_or("math")
    }

    /// The subject a principle belongs to.
    pub fn principle_subject(&self, principle: &str) -> &str {
        self.principle(principle).map(|p| self.skill_subject(&p.skill)).unwrap_or("math")
    }

    /// The domain of the subject a question counts toward.
    pub fn question_domain(&self, q: &Question) -> String {
        self.skill(&q.skill).map(|s| ged_content::question_domain(q, s)).unwrap_or_default()
    }

    /// The stages of one subject's week path, in order.
    pub fn path(&self, subject: &str) -> Vec<(usize, &Stage)> {
        self.journey.iter().enumerate().filter(|(_, s)| s.subject == subject).collect()
    }

    /// Whether learners may be served this question.
    pub fn servable(q: &Question, serve_drafts: bool) -> bool {
        status_servable(q.status, serve_drafts)
    }

    /// Whether learners may read this principle's lesson. The name and
    /// summary are always shown.
    pub fn lesson_servable(p: &Principle, serve_drafts: bool) -> bool {
        status_servable(p.status, serve_drafts)
    }

    /// Upsert skills and questions into SQLite; retire questions that are
    /// no longer in the files.
    pub async fn sync(&self, pool: &SqlitePool) -> sqlx::Result<()> {
        let mut tx = pool.begin().await?;
        for s in &self.skills {
            sqlx::query(
                "INSERT INTO skills (id, subject, area, name, description) VALUES (?, ?, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET subject = excluded.subject, area = excluded.area,
                     name = excluded.name, description = excluded.description",
            )
            .bind(&s.id)
            .bind(&s.subject)
            .bind(&s.area)
            .bind(&s.name)
            .bind(s.description.trim())
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query("UPDATE principles SET status = 'retired'").execute(&mut *tx).await?;
        for p in &self.principles {
            sqlx::query(
                "INSERT INTO principles (id, skill_id, name, status) VALUES (?, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET skill_id = excluded.skill_id, name = excluded.name,
                     status = excluded.status",
            )
            .bind(&p.id)
            .bind(&p.skill)
            .bind(&p.name)
            .bind(status_name(p.status))
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query("UPDATE questions SET status = 'retired'").execute(&mut *tx).await?;
        for q in self.questions.values() {
            sqlx::query(
                "INSERT INTO questions (id, skill_id, principle_id, difficulty, type, status, content_hash)
                 VALUES (?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET skill_id = excluded.skill_id,
                     principle_id = excluded.principle_id, difficulty = excluded.difficulty,
                     type = excluded.type, status = excluded.status, content_hash = excluded.content_hash",
            )
            .bind(&q.id)
            .bind(&q.skill)
            .bind(&q.principle)
            .bind(q.difficulty as i64)
            .bind(q.kind.name())
            .bind(status_name(q.status))
            .bind(&self.hashes[&q.id])
            .execute(&mut *tx)
            .await?;
        }
        // A flag on a question that has since been edited is settled.
        sqlx::query(
            "UPDATE flags SET resolved_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), resolution = 'question changed'
             WHERE resolved_at IS NULL
               AND content_hash != (SELECT q.content_hash FROM questions q WHERE q.id = flags.question_id)",
        )
        .execute(&mut *tx)
        .await?;
        // Attempts made before principles existed take their question's principle.
        sqlx::query(
            "UPDATE attempts SET principle_id =
                 (SELECT q.principle_id FROM questions q WHERE q.id = attempts.question_id)
             WHERE principle_id IS NULL",
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await
    }
}

fn status_servable(s: Status, serve_drafts: bool) -> bool {
    match s {
        Status::Reviewed => true,
        Status::Draft => serve_drafts,
        Status::Retired => false,
    }
}

fn status_name(s: Status) -> &'static str {
    match s {
        Status::Draft => "draft",
        Status::Reviewed => "reviewed",
        Status::Retired => "retired",
    }
}

pub use ged_content::review::hex;

/// Grade a learner's response.
///
/// - Multiple choice needs the exact choice.
/// - Fill-in answers compare as numbers when both sides parse (so "88.2"
///   matches "88.20"), otherwise as text ignoring spaces and case, against
///   the answer or any `accept` entry.
/// - Drop-down: a JSON list with one choice per menu, all right.
/// - Order: a JSON list of the items in the learner's order.
/// - Sort: a JSON object from each item to the group it was put in.
pub fn grade(q: &Question, response: &str) -> bool {
    match q.kind {
        QuestionType::MultipleChoice => response.trim() == q.answer.trim(),
        QuestionType::FillIn => std::iter::once(&q.answer).chain(q.accept.iter()).any(|want| {
            match (ged_content::parse_answer(want), ged_content::parse_answer(response)) {
                (Some(w), Some(got)) => ged_content::close_enough(w, got),
                _ => normalize(response) == normalize(want),
            }
        }),
        QuestionType::Dropdown => match serde_json::from_str::<Vec<String>>(response) {
            Ok(picks) => {
                picks.len() == q.blanks.len() && picks.iter().zip(&q.blanks).all(|(p, b)| p.trim() == b.answer.trim())
            }
            Err(_) => false,
        },
        QuestionType::Order => match serde_json::from_str::<Vec<String>>(response) {
            Ok(order) => order.len() == q.items.len() && order.iter().zip(&q.items).all(|(a, b)| a.trim() == b.trim()),
            Err(_) => false,
        },
        QuestionType::Sort => match serde_json::from_str::<HashMap<String, String>>(response) {
            Ok(placed) => {
                let total: usize = q.groups.iter().map(|g| g.items.len()).sum();
                placed.len() == total
                    && q.groups.iter().all(|g| g.items.iter().all(|i| placed.get(i.trim()).is_some_and(|got| got.trim() == g.name.trim())))
            }
            Err(_) => false,
        },
    }
}

fn normalize(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).flat_map(char::to_lowercase).collect()
}
