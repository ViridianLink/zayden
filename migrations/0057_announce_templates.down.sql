ALTER TABLE patreon_announce
    DROP COLUMN IF EXISTS ping_role_ids,
    DROP COLUMN IF EXISTS message;

ALTER TABLE youtube_announce
    DROP COLUMN IF EXISTS ping_role_ids,
    DROP COLUMN IF EXISTS message;

