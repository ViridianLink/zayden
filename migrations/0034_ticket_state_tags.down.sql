ALTER TABLE support_settings
    DROP COLUMN IF EXISTS fixed_tag_id,
    DROP COLUMN IF EXISTS closed_tag_id;

