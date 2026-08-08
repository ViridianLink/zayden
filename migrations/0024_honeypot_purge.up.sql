ALTER TABLE honeypot_settings
    ADD COLUMN purge_seconds integer NOT NULL DEFAULT 86400 CONSTRAINT honeypot_settings_purge_seconds_range CHECK (purge_seconds BETWEEN 0 AND 604800);

