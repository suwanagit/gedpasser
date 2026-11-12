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
    /// The subject this skill belongs to (`subjects.toml`). Math if left out.
    #[serde(default = "math")]
    pub subject: String,
    /// Math only: the domain every question in this skill counts toward,
    /// "quantitative" or "algebraic". Other subjects take the domain from
    /// each question's `topic`.
    #[serde(default)]
    pub area: String,
    pub name: String,
    pub description: String,
    pub difficulty_min: u8,
    pub difficulty_max: u8,
    pub targets: Vec<String>,
}

pub(crate) fn math() -> String {
    "math".into()
}

/// `content/subjects.toml`: the GED tests the app covers.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubjectFile {
    pub subject: Vec<Subject>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    pub id: String,
    pub name: String,
    /// Whether the real test allows a calculator throughout.
    #[serde(default)]
    pub calculator: bool,
    /// Content domains and their share of the real test. Shares sum to 1.
    pub domain: Vec<Domain>,
    /// Practice-test sizes: short, long and full.
    pub test: Vec<TestShape>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Domain {
    /// For Math, a skill `area`. Otherwise the prefix of a question's
    /// `topic` code, e.g. "L" for "L.c.2".
    pub id: String,
    pub name: String,
    pub share: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestShape {
    /// "short" | "long" | "full".
    pub kind: String,
    pub questions: u32,
    pub minutes: u32,
}

impl Subject {
    /// The built-in Math subject, used when a bank has no `subjects.toml`.
    pub fn default_math() -> Subject {
        let shape = |kind: &str, questions, minutes| TestShape { kind: kind.into(), questions, minutes };
        Subject {
            id: "math".into(),
            name: "Math".into(),
            calculator: false,
            domain: vec![
                Domain { id: "quantitative".into(), name: "Quantitative problem solving".into(), share: 0.45 },
                Domain { id: "algebraic".into(), name: "Algebraic problem solving".into(), share: 0.55 },
            ],
            test: vec![shape("short", 10, 25), shape("long", 25, 60), shape("full", 46, 115)],
        }
    }
}

/// `content/stimuli/<subject>.toml`: a text, table or figure that several
/// questions share.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StimulusFile {
    #[serde(default)]
    pub stimulus: Vec<Stimulus>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stimulus {
    pub id: String,
    pub status: Status,
    #[serde(default)]
    pub title: Option<String>,
    /// Paragraphs separated by blank lines; math in $...$.
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub table: Option<Table>,
    #[serde(default)]
    pub figure: Option<Figure>,
    /// Where the text or data comes from ("Original" for our own writing).
    pub source: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
#[serde(deny_unknown_fields)]
pub struct Table {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// One drop-down menu in a `dropdown` question.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Blank {
    pub choices: Vec<String>,
    pub answer: String,
}

/// One group in a `sort` question, with the items that belong in it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub name: String,
    pub items: Vec<String>,
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
    /// For a dropdown question, the text holds `{{1}}`, `{{2}}` … where the
    /// menus go.
    pub prompt: String,
    /// Options for multiple_choice; empty otherwise.
    #[serde(default)]
    pub choices: Vec<String>,
    /// The correct answer exactly as shown to (or typed by) the learner.
    /// Required for multiple_choice and fill_in; the other types carry
    /// their answers in `blanks`, `items` or `groups`.
    #[serde(default)]
    pub answer: String,
    /// fill_in: other answers that are also right ("the mitochondria").
    #[serde(default)]
    pub accept: Vec<String>,
    /// dropdown: one entry per `{{n}}` marker, in order.
    #[serde(default)]
    pub blanks: Vec<Blank>,
    /// order: the items in their correct order.
    #[serde(default)]
    pub items: Vec<String>,
    /// sort: the groups and the items that belong in each.
    #[serde(default)]
    pub groups: Vec<Group>,
    /// A shared stimulus from `stimuli/<subject>.toml`.
    #[serde(default)]
    pub stimulus: Option<String>,
    /// Official content-topic code, e.g. "L.c.2". Its first part names the
    /// subject domain the question counts toward.
    #[serde(default)]
    pub topic: Option<String>,
    /// Citations for facts the question relies on.
    #[serde(default)]
    pub sources: Vec<String>,
    /// Arithmetic expression the answer checker evaluates. Required for
    /// Math and for any numeric answer.
    #[serde(default)]
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
    /// The subject whose path this stage is on. Math if left out.
    #[serde(default = "math")]
    pub subject: String,
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
    /// Menus set inside a text; every menu must be right.
    Dropdown,
    /// Put the items in order.
    Order,
    /// Sort the items into groups.
    Sort,
}

impl QuestionType {
    pub fn name(self) -> &'static str {
        match self {
            QuestionType::MultipleChoice => "multiple_choice",
            QuestionType::FillIn => "fill_in",
            QuestionType::Dropdown => "dropdown",
            QuestionType::Order => "order",
            QuestionType::Sort => "sort",
        }
    }
}

impl Question {
    /// The correct answer as one readable line, for any question type.
    pub fn answer_text(&self) -> String {
        match self.kind {
            QuestionType::MultipleChoice | QuestionType::FillIn => self.answer.trim().to_string(),
            QuestionType::Dropdown => self.blanks.iter().map(|b| b.answer.trim()).collect::<Vec<_>>().join(" / "),
            QuestionType::Order => self.items.iter().map(|i| i.trim()).collect::<Vec<_>>().join(" → "),
            QuestionType::Sort => self
                .groups
                .iter()
                .map(|g| format!("{}: {}", g.name.trim(), g.items.iter().map(|i| i.trim()).collect::<Vec<_>>().join(", ")))
                .collect::<Vec<_>>()
                .join("; "),
        }
    }
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
    /// A shared stimulus from `content/stimuli/<subject>.toml`.
    Stimulus,
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
