DROP INDEX support_thread_activity_unstale_idx;

DROP INDEX support_thread_activity_stale_idx;

ALTER TABLE support_thread_activity
    DROP COLUMN staled_at;

ALTER TABLE support_settings
    DROP COLUMN stale_enabled,
    DROP COLUMN stale_tag_id,
    DROP COLUMN stale_after_secs;

