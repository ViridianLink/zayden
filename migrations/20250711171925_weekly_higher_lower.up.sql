-- Add up migration script here
ALTER TABLE gambling_stats
ADD COLUMN weekly_higher_or_lower_score INTEGER NOT NULL DEFAULT 0