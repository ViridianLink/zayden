DROP TABLE IF EXISTS guild_helper_links;

ALTER TABLE support_settings
    DROP COLUMN IF EXISTS solved_tag_id,
    DROP COLUMN IF EXISTS helper_role_id,
    DROP COLUMN IF EXISTS solved_archive_secs;

