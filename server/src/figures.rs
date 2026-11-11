//! Figures for the browser: the figure as written, plus curves sampled into
//! polylines so the client draws without evaluating expressions.

use ged_content::Figure;
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct FigureView {
    pub figure: Figure,
    /// For a `plane`, each curve as one or more polylines of [x, y] points
    /// (split where the curve leaves the window or is undefined).
    #[ts(type = "Array<Array<Array<[number, number]>>>")]
    pub curves: Vec<Vec<Vec<[f64; 2]>>>,
}

const SAMPLES: usize = 240;

impl FigureView {
    pub fn new(figure: &Figure) -> FigureView {
        let curves = match figure {
            Figure::Plane { x_min, x_max, y_min, y_max, curves, .. } => {
                curves.iter().map(|c| sample(c, *x_min, *x_max, *y_min, *y_max)).collect()
            }
            _ => Vec::new(),
        };
        FigureView { figure: figure.clone(), curves }
    }
}

/// Sample `expr` over [x_min, x_max]. Points far outside the y window break
/// the line so asymptotes don't draw vertical streaks.
fn sample(expr: &str, x_min: f64, x_max: f64, y_min: f64, y_max: f64) -> Vec<Vec<[f64; 2]>> {
    let span = y_max - y_min;
    let (lo, hi) = (y_min - span, y_max + span);
    let mut lines = Vec::new();
    let mut current: Vec<[f64; 2]> = Vec::new();
    for i in 0..=SAMPLES {
        let x = x_min + (x_max - x_min) * i as f64 / SAMPLES as f64;
        match ged_content::expr::eval_x(expr, x) {
            Ok(y) if y.is_finite() && y >= lo && y <= hi => current.push([round(x), round(y)]),
            _ => {
                if current.len() > 1 {
                    lines.push(std::mem::take(&mut current));
                } else {
                    current.clear();
                }
            }
        }
    }
    if current.len() > 1 {
        lines.push(current);
    }
    lines
}

fn round(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_a_line_and_breaks_at_gaps() {
        let line = sample("2*x - 3", -5.0, 5.0, -6.0, 6.0);
        assert_eq!(line.len(), 1);
        assert!(line[0].len() > 50);
        // sqrt is undefined for x < 0: one piece, starting at x ≥ 0.
        let root = sample("sqrt(x)", -4.0, 4.0, -1.0, 3.0);
        assert_eq!(root.len(), 1);
        assert!(root[0][0][0] >= 0.0);
    }
}
