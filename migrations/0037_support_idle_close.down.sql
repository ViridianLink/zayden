DROP INDEX support_thread_activity_close_idx;

ALTER TABLE support_settings
    DROP COLUMN idle_close_enabled,
    DROP COLUMN idle_close_after_secs;

