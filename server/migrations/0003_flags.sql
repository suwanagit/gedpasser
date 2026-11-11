-- Learner reports of problems with a question. A question with any open
-- flag is taken out of rotation for everyone until the flag is resolved:
-- automatically when the question's content changes, or by hand with
-- `gedpasser-server flags resolve <id>`.
CREATE TABLE flags (
    id           INTEGER PRIMARY KEY,
    user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    question_id  TEXT    NOT NULL REFERENCES questions(id),
    content_hash TEXT    NOT NULL,           -- question version that was flagged
    reason       TEXT    NOT NULL,           -- wrong_answer | unclear | typo | other
    note         TEXT    NOT NULL DEFAULT '',
    created_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    resolved_at  TEXT,
    resolution   TEXT                        -- 'question changed' | 'dismissed'
);
CREATE INDEX flags_open ON flags(question_id) WHERE resolved_at IS NULL;
