ALTER TABLE support_settings
    ADD COLUMN idle_close_enabled boolean NOT NULL DEFAULT FALSE,
    ADD COLUMN idle_close_after_secs int NOT NULL DEFAULT 86400 CHECK (idle_close_after_secs >= 3600);

CREATE INDEX support_thread_activity_close_idx ON support_thread_activity (guild_id, nudged_at)
WHERE
    nudged_at IS NOT NULL AND NOT paused AND NOT waiting_on_helper;

