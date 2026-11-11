//! Mastery: how well a learner knows a skill or principle, computed from
//! their recent answers. Never stored, so the formula can change freely.
//!
//! mastery = (Σ w·credit + PRIOR_RIGHT) / (Σ w + PRIOR_TOTAL)
//! where w = DECAY^k for the k-th most recent answer (k = 0 newest), over the
//! last WINDOW answers, and credit is 1 (correct), HINT_CREDIT (correct after
//! opening the lesson) or 0 (wrong). The prior acts like one imaginary right
//! and one imaginary wrong answer, so an untried skill sits at 0.5 and one
//! lucky answer can't show 100%.

use serde::Serialize;
use ts_rs::TS;

pub const WINDOW: usize = 20;
pub const DECAY: f64 = 0.85;
pub const HINT_CREDIT: f64 = 0.5;
const PRIOR_RIGHT: f64 = 1.0;
const PRIOR_TOTAL: f64 = 2.0;

/// One answer, newest first when passed to [`mastery`].
#[derive(Debug, Clone, Copy)]
pub struct Answer {
    pub correct: bool,
    pub hint_used: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    NotStarted,
    NeedsWork,
    GettingThere,
    Strong,
}

/// Mastery from answers ordered newest first. Only the first [`WINDOW`]
/// answers count.
pub fn mastery(newest_first: &[Answer]) -> f64 {
    let mut weighted = 0.0;
    let mut total = 0.0;
    for (k, a) in newest_first.iter().take(WINDOW).enumerate() {
        let w = DECAY.powi(k as i32);
        let credit = match (a.correct, a.hint_used) {
            (true, false) => 1.0,
            (true, true) => HINT_CREDIT,
            (false, _) => 0.0,
        };
        weighted += w * credit;
        total += w;
    }
    (weighted + PRIOR_RIGHT) / (total + PRIOR_TOTAL)
}

pub fn level(answered: usize, mastery: f64) -> Level {
    if answered == 0 {
        Level::NotStarted
    } else if mastery < 0.5 {
        Level::NeedsWork
    } else if mastery < 0.8 {
        Level::GettingThere
    } else {
        Level::Strong
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answers(pattern: &str) -> Vec<Answer> {
        // newest first: 'c' correct, 'h' correct with hint, 'w' wrong
        pattern
            .chars()
            .map(|c| Answer { correct: c != 'w', hint_used: c == 'h' })
            .collect()
    }

    #[test]
    fn untried_is_half() {
        assert_eq!(mastery(&[]), 0.5);
        assert_eq!(level(0, 0.5), Level::NotStarted);
    }

    #[test]
    fn all_correct_climbs_but_never_reaches_one() {
        let one = mastery(&answers("c"));
        let five = mastery(&answers("ccccc"));
        let twenty = mastery(&answers(&"c".repeat(20)));
        assert!(one < five && five < twenty && twenty < 1.0);
        assert!(one < 0.8, "one lucky answer is not 'strong': {one}");
        assert_eq!(level(20, twenty), Level::Strong);
    }

    #[test]
    fn recent_answers_count_more() {
        let recent_wrong = mastery(&answers("wwcccccc"));
        let old_wrong = mastery(&answers("ccccccww"));
        assert!(recent_wrong < old_wrong);
    }

    #[test]
    fn hints_count_less_and_wrong_counts_nothing() {
        assert!(mastery(&answers("hhhh")) < mastery(&answers("cccc")));
        assert!(mastery(&answers("wwww")) < mastery(&answers("hhhh")));
        assert_eq!(level(4, mastery(&answers("wwww"))), Level::NeedsWork);
    }

    #[test]
    fn only_the_window_counts() {
        let mut a = answers(&"c".repeat(WINDOW));
        let base = mastery(&a);
        a.extend(answers(&"w".repeat(30)));
        assert_eq!(mastery(&a), base);
    }
}
