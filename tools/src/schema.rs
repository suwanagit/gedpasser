//! Serde types for `content/skills.toml` and `content/math/<skill>.toml`.
//! Unknown fields are rejected so typos in question files fail loudly.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillFile {
    pub skill: Vec<Skill>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Skill {
    pub id: String,
    pub area: Area,
    pub name: String,
    pub description: String,
    pub difficulty_min: u8,
    pub difficulty_max: u8,
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Area {
    Quantitative,
    Algebraic,
}

/// `content/principles.toml`: one principle per official assessment
/// indicator. Names and lessons are our own wording.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrincipleFile {
    pub principle: Vec<Principle>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Principle {
    /// Official indicator code, e.g. "Q.3.d".
    pub id: String,
    pub skill: String,
    /// Status of the lesson text: draft | reviewed | retired.
    pub status: Status,
    pub name: String,
    /// One sentence shown in lists.
    pub summary: String,
    /// Paragraphs separated by blank lines; "- " lines are bullets; math in $...$.
    pub lesson: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestionFile {
    #[serde(default)]
    pub question: Vec<Question>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Question {
    pub id: String,
    pub skill: String,
    /// The principle (official indicator code) this question tests; must
    /// belong to `skill`.
    pub principle: String,
    /// 1 easy to 3 hard.
    pub difficulty: u8,
    #[serde(rename = "type")]
    pub kind: QuestionType,
    pub status: Status,
    #[serde(default)]
    pub alaska: bool,
    pub prompt: String,
    /// Options for multiple_choice and dropdown; empty for fill_in.
    #[serde(default)]
    pub choices: Vec<String>,
    /// The correct answer exactly as shown to (or typed by) the learner.
    pub answer: String,
    /// Arithmetic expression the answer checker evaluates.
    pub check: String,
    /// Numeric value to compare `check` against when `answer` is not a
    /// plain number (for example an algebraic expression).
    #[serde(default)]
    pub answer_value: Option<f64>,
    pub explanation: String,
    pub common_mistake: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionType {
    MultipleChoice,
    FillIn,
    Dropdown,
    DragDrop,
}

/// `content/reviews/<skill>.toml`: notes from an independent reviewer.
/// Later entries for the same item replace earlier ones.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewFile {
    #[serde(default)]
    pub review: Vec<Review>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    /// Question id, or principle code for a lesson.
    pub id: String,
    pub kind: ReviewKind,
    /// Content hash of the item the reviewer saw (`ged-content template`).
    pub hash: String,
    pub verdict: Verdict,
    /// The reviewer's own answer, worked out before reading the key.
    #[serde(default)]
    pub solved: Option<String>,
    pub note: String,
    pub date: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewKind {
    Question,
    Lesson,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Fix,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Draft,
    Reviewed,
    Retired,
}
