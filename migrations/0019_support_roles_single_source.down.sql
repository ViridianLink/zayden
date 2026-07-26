ALTER TABLE support_settings
    ADD COLUMN support_role_id bigint,
    ADD COLUMN support_thread_id int NOT NULL DEFAULT 0;

UPDATE
    support_settings s
SET
    support_role_id =(
        SELECT
            r.role_id
        FROM
            guild_support_roles r
        WHERE
            r.guild_id = s.guild_id
        ORDER BY
            r.role_id
        LIMIT 1);

