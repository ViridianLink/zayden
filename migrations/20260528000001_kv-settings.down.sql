DROP TABLE IF EXISTS guild_settings_kv;

ALTER TABLE guild_config
    DROP COLUMN IF EXISTS updated_at;
