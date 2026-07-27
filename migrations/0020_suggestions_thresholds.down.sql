ALTER TABLE suggestions_settings
    DROP CONSTRAINT suggestions_settings_threshold_order;

ALTER TABLE suggestions_settings
    DROP COLUMN promote_threshold,
    DROP COLUMN demote_threshold;

