ALTER TABLE support_settings
    ADD COLUMN stale_enabled boolean NOT NULL DEFAULT FALSE,
    ADD COLUMN stale_tag_id bigint,
    ADD COLUMN stale_after_secs int NOT NULL DEFAULT 604800 CHECK (stale_after_secs >= 3600);

ALTER TABLE support_thread_activity
    ADD COLUMN staled_at timestamptz;

CREATE INDEX support_thread_activity_stale_idx ON support_thread_activity (guild_id, since)
WHERE
    staled_at IS NULL AND NOT paused AND NOT waiting_on_helper;

CREATE INDEX support_thread_activity_unstale_idx ON support_thread_activity (thread_id)
WHERE
    staled_at IS NOT NULL;

