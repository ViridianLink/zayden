-- Single-row table for deployment-level runtime overrides.
-- At most one row is enforced by CHECK (id = 1) with DEFAULT 1.
-- Insert a row to activate overrides: INSERT INTO bot_config DEFAULT VALUES;
CREATE TABLE bot_config (
    id           SMALLINT PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    error_log_webhook  TEXT,
    normal_log_webhook TEXT,
    feature_flags      JSONB NOT NULL DEFAULT '{}'
);
