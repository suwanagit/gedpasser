-- Journey, spaced review, practice tests, and the learner's test date.

-- One attempt at a journey stage: the questions served, and the result.
CREATE TABLE journey_runs (
    id           INTEGER PRIMARY KEY,
    user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    stage_id     TEXT    NOT NULL,
    question_ids TEXT    NOT NULL,          -- JSON array
    started_at   INTEGER NOT NULL,          -- unix seconds
    finished_at  INTEGER,
    correct      INTEGER,
    total        INTEGER
);
CREATE INDEX journey_runs_user ON journey_runs(user_id);

-- Best result per stage. A stage is passed once; stars can improve.
CREATE TABLE journey_progress (
    user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    stage_id     TEXT    NOT NULL,
    best_correct INTEGER NOT NULL,
    best_total   INTEGER NOT NULL,
    stars        INTEGER NOT NULL,          -- 0 not passed, 1–3 passed
    completed_at INTEGER NOT NULL,
    PRIMARY KEY (user_id, stage_id)
);

-- Spaced review: missed questions come back sooner, mastered ones later.
CREATE TABLE review_queue (
    user_id       INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    question_id   TEXT    NOT NULL REFERENCES questions(id),
    due_at        INTEGER NOT NULL,         -- unix seconds
    interval_days INTEGER NOT NULL,
    PRIMARY KEY (user_id, question_id)
);
CREATE INDEX review_queue_due ON review_queue(user_id, due_at);

-- Practice tests (short, long, full length).
CREATE TABLE tests (
    id              INTEGER PRIMARY KEY,
    user_id         INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind            TEXT    NOT NULL,       -- short | long | full
    question_ids    TEXT    NOT NULL,       -- JSON array
    time_limit_secs INTEGER NOT NULL,
    started_at      INTEGER NOT NULL,
    finished_at     INTEGER,
    correct         INTEGER,
    total           INTEGER
);
CREATE INDEX tests_user ON tests(user_id);

ALTER TABLE users ADD COLUMN test_date TEXT;  -- YYYY-MM-DD, learner's GED date
