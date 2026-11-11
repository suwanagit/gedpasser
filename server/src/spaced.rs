//! Spaced review: a missed question comes back soon; each correct answer
//! after that pushes it further out, until it drops off the queue.

use sqlx::SqlitePool;

/// Days until the next review after each correct answer (0 = 10 minutes).
pub const INTERVALS: [i64; 6] = [0, 1, 3, 7, 14, 30];
const FIRST_RETRY_SECS: i64 = 10 * 60;
const DAY: i64 = 24 * 60 * 60;

pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn due_after(interval_days: i64) -> i64 {
    if interval_days == 0 { FIRST_RETRY_SECS } else { interval_days * DAY }
}

/// Update the queue after an answer.
pub async fn record(pool: &SqlitePool, user_id: i64, question_id: &str, correct: bool) -> sqlx::Result<()> {
    let now = now_unix();
    let current: Option<i64> =
        sqlx::query_scalar("SELECT interval_days FROM review_queue WHERE user_id = ? AND question_id = ?")
            .bind(user_id)
            .bind(question_id)
            .fetch_optional(pool)
            .await?;
    let next = match (correct, current) {
        // Missed: back in ten minutes, schedule restarts.
        (false, _) => Some(0),
        // Right but never missed: nothing to review.
        (true, None) => None,
        // Right again: step out; past the last interval, it's learned.
        (true, Some(i)) => INTERVALS.iter().copied().find(|&n| n > i),
    };
    match next {
        Some(interval) => {
            sqlx::query(
                "INSERT INTO review_queue (user_id, question_id, due_at, interval_days) VALUES (?, ?, ?, ?)
                 ON CONFLICT(user_id, question_id) DO UPDATE SET due_at = excluded.due_at,
                     interval_days = excluded.interval_days",
            )
            .bind(user_id)
            .bind(question_id)
            .bind(now + due_after(interval))
            .bind(interval)
            .execute(pool)
            .await?;
        }
        None => {
            if current.is_some() {
                sqlx::query("DELETE FROM review_queue WHERE user_id = ? AND question_id = ?")
                    .bind(user_id)
                    .bind(question_id)
                    .execute(pool)
                    .await?;
            }
        }
    }
    Ok(())
}

/// Question ids due for review now, soonest first.
pub async fn due_ids(pool: &SqlitePool, user_id: i64) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar("SELECT question_id FROM review_queue WHERE user_id = ? AND due_at <= ? ORDER BY due_at")
        .bind(user_id)
        .bind(now_unix())
        .fetch_all(pool)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intervals_grow_and_end() {
        let steps: Vec<i64> = std::iter::successors(Some(0i64), |&i| INTERVALS.iter().copied().find(|&n| n > i)).collect();
        assert_eq!(steps, vec![0, 1, 3, 7, 14, 30]);
        assert_eq!(due_after(0), 600);
        assert_eq!(due_after(3), 3 * DAY);
    }
}
