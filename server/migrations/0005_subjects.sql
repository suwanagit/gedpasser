-- Phase 7: more than one GED subject.
ALTER TABLE skills ADD COLUMN subject TEXT NOT NULL DEFAULT 'math';
ALTER TABLE tests ADD COLUMN subject TEXT NOT NULL DEFAULT 'math';
