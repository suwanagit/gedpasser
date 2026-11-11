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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Draft,
    Reviewed,
    Retired,
}
