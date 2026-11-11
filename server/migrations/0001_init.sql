-- Phase 1: profiles, sessions, the synced question bank, and attempts.
-- Times are ISO-8601 UTC text, except session expiry (unix seconds) which is
-- compared numerically.

CREATE TABLE users (
    id            INTEGER PRIMARY KEY,
    display_name  TEXT    NOT NULL UNIQUE COLLATE NOCASE,
    password_hash TEXT    NOT NULL,          -- Argon2id PHC string
    created_at    TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE sessions (
    token_hash TEXT    PRIMARY KEY,          -- SHA-256 of the cookie token; the token itself is never stored
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    expires_at INTEGER NOT NULL              -- unix seconds
);
CREATE INDEX sessions_user ON sessions(user_id);

-- Synced from content/skills.toml at startup.
CREATE TABLE skills (
    id          TEXT PRIMARY KEY,
    area        TEXT NOT NULL,
    name        TEXT NOT NULL,
    description TEXT NOT NULL
);

-- Synced from content/math/*.toml at startup. Questions that disappear from
-- the files are marked 'retired', never deleted, so attempts stay valid.
CREATE TABLE questions (
    id           TEXT    PRIMARY KEY,
    skill_id     TEXT    NOT NULL REFERENCES skills(id),
    difficulty   INTEGER NOT NULL,
    type         TEXT    NOT NULL,
    status       TEXT    NOT NULL,
    content_hash TEXT    NOT NULL
);
CREATE INDEX questions_skill ON questions(skill_id);

CREATE TABLE attempts (
    id           INTEGER PRIMARY KEY,
    user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    question_id  TEXT    NOT NULL REFERENCES questions(id),
    response     TEXT    NOT NULL,           -- what the learner answered
    correct      INTEGER NOT NULL,           -- 0 or 1
    time_ms      INTEGER NOT NULL,
    mode         TEXT    NOT NULL,           -- 'practice' for now
    content_hash TEXT    NOT NULL,           -- question version that was answered
    answered_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX attempts_user_question ON attempts(user_id, question_id);
