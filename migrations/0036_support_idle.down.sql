DROP TABLE support_thread_activity;

ALTER TABLE support_settings
    DROP COLUMN idle_enabled,
    DROP COLUMN idle_after_secs,
    ADD COLUMN helper_role_id bigint;

UPDATE
    support_settings s
SET
    helper_role_id = (
        SELECT
            r.role_id
        FROM
            guild_support_roles r
        WHERE
            r.guild_id = s.guild_id
        ORDER BY
            r.role_id
        LIMIT 1);

