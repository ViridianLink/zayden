ALTER TABLE suggestions_settings
    ADD COLUMN promote_threshold integer NOT NULL DEFAULT 20,
    ADD COLUMN demote_threshold integer NOT NULL DEFAULT 15;

ALTER TABLE suggestions_settings
    ADD CONSTRAINT suggestions_settings_threshold_order CHECK (demote_threshold < promote_threshold);

