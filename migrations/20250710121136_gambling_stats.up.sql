CREATE TABLE
    gambling_stats (
        user_id BIGINT PRIMARY KEY,
        max_cash BIGINT NOT NULL DEFAULT 0,
        total_cash BIGINT NOT NULL DEFAULT 0,
        gifts_given INTEGER NOT NULL DEFAULT 0,
        gifts_received INTEGER NOT NULL DEFAULT 0,
        higher_or_lower_score INTEGER NOT NULL DEFAULT 0
    );

CREATE INDEX idx_gambling_max_cash ON gambling_stats (max_cash DESC);

CREATE INDEX idx_gambling_total_cash ON gambling_stats (total_cash DESC);

CREATE INDEX idx_gambling_gifts_given ON gambling_stats (gifts_given DESC);

CREATE INDEX idx_gambling_gifts_received ON gambling_stats (gifts_received DESC);

CREATE INDEX idx_gambling_higher_or_lower_score ON gambling_stats (higher_or_lower_score DESC);