-- Principles (one per official assessment indicator), synced from
-- content/principles.toml. Lessons themselves stay in the file.
CREATE TABLE principles (
    id       TEXT PRIMARY KEY,                 -- official indicator code, e.g. 'Q.3.d'
    skill_id TEXT NOT NULL REFERENCES skills(id),
    name     TEXT NOT NULL,
    status   TEXT NOT NULL                     -- lesson status: draft | reviewed | retired
);

ALTER TABLE questions ADD COLUMN principle_id TEXT REFERENCES principles(id);

-- The principle a question tested when it was answered (kept even if the
-- question is later re-tagged), and whether the learner opened the lesson
-- before answering.
ALTER TABLE attempts ADD COLUMN principle_id TEXT REFERENCES principles(id);
ALTER TABLE attempts ADD COLUMN hint_used INTEGER NOT NULL DEFAULT 0;
CREATE INDEX attempts_user_principle ON attempts(user_id, principle_id);
