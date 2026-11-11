//! The question bank in memory, plus its sync into SQLite at startup.
//! The TOML files stay the source of truth; the database only holds ids,
//! metadata and content hashes so attempts can reference questions.

use std::collections::HashMap;
use std::path::Path;

use ged_content::{Question, QuestionType, Skill, Status, check_content};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

pub struct Bank {
    pub skills: Vec<Skill>,
    pub questions: HashMap<String, Question>,
    /// Question ids per skill, in file order.
    pub by_skill: HashMap<String, Vec<String>>,
    pub hashes: HashMap<String, String>,
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
        Ok(Bank { skills: report.skills, questions, by_skill, hashes })
    }

    /// Whether learners may be served this question.
    pub fn servable(q: &Question, serve_drafts: bool) -> bool {
        match q.status {
            Status::Reviewed => true,
            Status::Draft => serve_drafts,
            Status::Retired => false,
        }
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
        sqlx::query("UPDATE questions SET status = 'retired'").execute(&mut *tx).await?;
        for q in self.questions.values() {
            sqlx::query(
                "INSERT INTO questions (id, skill_id, difficulty, type, status, content_hash)
                 VALUES (?, ?, ?, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET skill_id = excluded.skill_id,
                     difficulty = excluded.difficulty, type = excluded.type,
                     status = excluded.status, content_hash = excluded.content_hash",
            )
            .bind(&q.id)
            .bind(&q.skill)
            .bind(q.difficulty as i64)
            .bind(type_name(q.kind))
            .bind(status_name(q.status))
            .bind(&self.hashes[&q.id])
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await
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

/// SHA-256 over what a learner sees and is graded on.
fn content_hash(q: &Question) -> String {
    let mut h = Sha256::new();
    for part in [type_name(q.kind), q.prompt.as_str(), &q.choices.join("\u{1f}"), q.answer.as_str()] {
        h.update(part.as_bytes());
        h.update([0u8]);
    }
    hex(&h.finalize())
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

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
