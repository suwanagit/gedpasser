//! Serde types for everything in `content/`. Unknown fields are rejected so
//! typos in content files fail loudly.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

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
    /// Optional graph, number line, chart or shape shown with the prompt.
    #[serde(default)]
    pub figure: Option<Figure>,
}

// ── Figures ──────────────────────────────────────────────────────────────
//
// Written in TOML as a sub-table, e.g.
//   [question.figure]
//   kind = "plane"
//   curves = ["2*x - 3"]
//   points = [{ x = 0, y = -3, label = "(0, −3)" }]

/// A figure drawn as SVG in the app. Curves are expressions in `x`.
#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Figure {
    /// Coordinate plane with optional curves, points and segments.
    Plane {
        #[serde(default = "neg_six")]
        x_min: f64,
        #[serde(default = "six")]
        x_max: f64,
        #[serde(default = "neg_six")]
        y_min: f64,
        #[serde(default = "six")]
        y_max: f64,
        #[serde(default)]
        curves: Vec<String>,
        #[serde(default)]
        points: Vec<Pt>,
        #[serde(default)]
        segments: Vec<Seg>,
        #[serde(default)]
        x_label: Option<String>,
        #[serde(default)]
        y_label: Option<String>,
    },
    /// Number line, e.g. for inequalities.
    NumberLine {
        min: f64,
        max: f64,
        #[serde(default)]
        points: Vec<LinePoint>,
        #[serde(default)]
        ray: Option<Ray>,
    },
    /// Bar graph.
    Bars {
        bars: Vec<Bar>,
        #[serde(default)]
        y_label: Option<String>,
    },
    /// Dot plot: a stack of `count` dots above each value.
    Dots {
        counts: Vec<DotCount>,
        #[serde(default)]
        label: Option<String>,
    },
    /// Box plot on a number line.
    Box {
        min: f64,
        q1: f64,
        median: f64,
        q3: f64,
        max: f64,
        axis_min: f64,
        axis_max: f64,
        #[serde(default)]
        label: Option<String>,
    },
    /// Geometry: polygons with side labels, circles, extra segments. No axes.
    Shape {
        #[serde(default)]
        polygons: Vec<Polygon>,
        #[serde(default)]
        circles: Vec<Circle>,
        #[serde(default)]
        segments: Vec<Seg>,
        #[serde(default)]
        labels: Vec<Pt>,
    },
}

fn six() -> f64 {
    6.0
}
fn neg_six() -> f64 {
    -6.0
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
#[serde(deny_unknown_fields)]
pub struct Pt {
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
#[serde(deny_unknown_fields)]
pub struct Seg {
    pub from: [f64; 2],
    pub to: [f64; 2],
    #[serde(default)]
    pub dashed: bool,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
#[serde(deny_unknown_fields)]
pub struct LinePoint {
    pub x: f64,
    /// Open circle (not included) instead of a filled one.
    #[serde(default)]
    pub open: bool,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
#[serde(deny_unknown_fields)]
pub struct Ray {
    pub from: f64,
    /// "left" or "right".
    pub dir: RayDir,
    #[serde(default)]
    pub open: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum RayDir {
    Left,
    Right,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
#[serde(deny_unknown_fields)]
pub struct Bar {
    pub label: String,
    pub value: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
#[serde(deny_unknown_fields)]
pub struct DotCount {
    pub x: f64,
    pub count: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
#[serde(deny_unknown_fields)]
pub struct Polygon {
    pub points: Vec<Pt>,
    /// Labels for sides in order (side i joins point i and i+1); "" for none.
    #[serde(default)]
    pub side_labels: Vec<String>,
    /// Indexes of vertices that get a right-angle mark.
    #[serde(default)]
    pub right_angles: Vec<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
#[serde(deny_unknown_fields)]
pub struct Circle {
    pub x: f64,
    pub y: f64,
    pub r: f64,
    /// Label drawn along a radius, e.g. "r = 3 cm".
    #[serde(default)]
    pub radius_label: Option<String>,
}

// ── Cards: the short lesson for each principle ───────────────────────────

/// `content/cards/<skill>.toml`: one short card per principle.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardFile {
    #[serde(default)]
    pub card: Vec<Card>,
}

/// A principle in as few words as possible: one rule, a few steps, one
/// worked example, one memory hack.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Card {
    /// Principle id (official indicator code).
    pub id: String,
    pub status: Status,
    /// 2–5 words.
    pub title: String,
    /// The one rule or formula to remember. Use $$...$$ for display math.
    pub key: String,
    /// 1–4 short steps.
    pub steps: Vec<String>,
    /// One short worked example.
    pub example: String,
    /// A memory device or test shortcut.
    pub hack: String,
    #[serde(default)]
    pub figure: Option<Figure>,
}

// ── Journey ──────────────────────────────────────────────────────────────

/// `content/journey.toml`: the guided path, in order.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JourneyFile {
    pub stage: Vec<Stage>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Stage {
    pub id: String,
    /// Day of the week path, 1–7.
    pub day: u8,
    pub title: String,
    /// Principles taught in this stage. Empty for a checkpoint, which mixes
    /// every principle since the previous checkpoint.
    #[serde(default)]
    pub principles: Vec<String>,
    #[serde(default)]
    pub checkpoint: bool,
    /// For a checkpoint: mix every principle taught so far, not just those
    /// since the previous checkpoint.
    #[serde(default)]
    pub cumulative: bool,
    /// Number of quiz questions.
    pub size: u32,
    /// Prefer questions up to this difficulty (1–3).
    pub max_difficulty: u8,
}

// ── Study: where to go learn each principle ───────────────────────────────

/// `content/study.toml`: for each principle, a topic to search for and one
/// or two free pages to study on your own. Reviewed as a whole.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StudyFile {
    pub status: Status,
    #[serde(default)]
    pub topic: Vec<StudyTopic>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StudyTopic {
    pub principle: String,
    /// A few words to search for, e.g. "percent word problems".
    pub search: String,
    #[serde(default)]
    pub links: Vec<StudyLink>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StudyLink {
    pub title: String,
    pub url: String,
}

// ── Strategy: high-yield ranking, test-day tactics, formula sheet ────────

/// `content/strategy.toml`. Reviewed as one item.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrategyFile {
    pub status: Status,
    /// Principles that come up most on the test, in priority order.
    pub high_yield: Vec<String>,
    /// Next most useful principles.
    pub medium_yield: Vec<String>,
    pub tactic: Vec<Tactic>,
    pub formula: Vec<Formula>,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
#[serde(deny_unknown_fields)]
pub struct Tactic {
    pub id: String,
    pub title: String,
    /// One or two short sentences.
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
#[serde(deny_unknown_fields)]
pub struct Formula {
    pub group: String,
    pub name: String,
    /// LaTeX, shown as display math.
    pub tex: String,
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
    Card,
    /// The whole of `content/strategy.toml` (id "strategy").
    Strategy,
    /// The whole of `content/study.toml` (id "study").
    Study,
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
