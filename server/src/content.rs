//! The question bank in memory, plus its sync into SQLite at startup.
//! The TOML files stay the source of truth; the database only holds ids,
//! metadata and content hashes so attempts can reference questions.

use std::collections::HashMap;
use std::path::Path;

use ged_content::{Card, Principle, Question, QuestionType, Skill, Stage, Status, StrategyFile, check_content};
use sqlx::SqlitePool;

pub struct Bank {
    pub skills: Vec<Skill>,
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
        for (_, q) in report.questions {
            by_skill.entry(q.skill.clone()).or_default().push(q.id.clone());
            hashes.insert(q.id.clone(), content_hash(&q));
            questions.insert(q.id.clone(), q);
        }
        let skill_order: HashMap<&str, usize> =
            report.skills.iter().enumerate().map(|(i, s)| (s.id.as_str(), i)).collect();
        let mut principles = report.principles.clone();
        principles.sort_by_key(|p| skill_order.get(p.skill.as_str()).copied().unwrap_or(usize::MAX));
        let cards = report.cards.into_iter().map(|(_, c)| (c.id.clone(), c)).collect();
        Ok(Bank {
            skills: report.skills,
            principles,
            questions,
            by_skill,
            hashes,
            cards,
            journey: report.journey,
            strategy: report.strategy,
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

    pub fn principle(&self, id: &str) -> Option<&Principle> {
        self.principles.iter().find(|p| p.id == id)
    }

    pub fn skill(&self, id: &str) -> Option<&Skill> {
        self.skills.iter().find(|s| s.id == id)
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
            let area = match s.area {
                ged_content::Area::Quantitative => "quantitative",
                ged_content::Area::Algebraic => "algebraic",
            };
            sqlx::query(
                "INSERT INTO skills (id, area, name, description) VALUES (?, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET area = excluded.area, name = excluded.name,
                     description = excluded.description",
            )
            .bind(&s.id)
            .bind(area)
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
            .bind(type_name(q.kind))
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

pub fn type_name(t: QuestionType) -> &'static str {
    match t {
        QuestionType::MultipleChoice => "multiple_choice",
        QuestionType::FillIn => "fill_in",
        QuestionType::Dropdown => "dropdown",
        QuestionType::DragDrop => "drag_drop",
    }
}

fn status_name(s: Status) -> &'static str {
    match s {
        Status::Draft => "draft",
        Status::Reviewed => "reviewed",
        Status::Retired => "retired",
    }
}

/// The same hash the review gate uses, so "changed since answered" and
/// "changed since reviewed" agree.
fn content_hash(q: &Question) -> String {
    ged_content::question_hash(q)
}

pub use ged_content::review::hex;

/// Grade a learner's response.
/// Choice questions need the exact choice. Fill-in answers compare as
/// numbers when both sides parse (so "88.2" matches "88.20"), otherwise as
/// text ignoring spaces and case.
pub fn grade(q: &Question, response: &str) -> bool {
    match q.kind {
        QuestionType::MultipleChoice | QuestionType::Dropdown => response.trim() == q.answer.trim(),
        QuestionType::FillIn => {
            match (ged_content::parse_answer(&q.answer), ged_content::parse_answer(response)) {
                (Some(want), Some(got)) => ged_content::close_enough(want, got),
                _ => normalize(response) == normalize(&q.answer),
            }
        }
        QuestionType::DragDrop => false,
    }
}

fn normalize(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).flat_map(char::to_lowercase).collect()
}
