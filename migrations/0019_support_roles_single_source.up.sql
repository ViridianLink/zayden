INSERT INTO guild_support_roles(guild_id, role_id)
SELECT
    guild_id,
    support_role_id
FROM
    support_settings
WHERE
    support_role_id IS NOT NULL
ON CONFLICT (guild_id,
    role_id)
    DO NOTHING;

ALTER TABLE support_settings
    DROP COLUMN support_role_id,
    DROP COLUMN support_thread_id;

