-- Add down migration script here
DROP INDEX idx_gambling_higher_or_lower_score;

DROP INDEX idx_gambling_gifts_received;

DROP INDEX idx_gambling_gifts_given;

DROP INDEX idx_gambling_total_cash;

DROP INDEX idx_gambling_max_cash;

DROP TABLE gambling_stats;