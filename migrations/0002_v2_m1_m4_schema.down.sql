-- Reverses 0002_v2_m1_m4_schema.up.sql.
DROP TABLE IF EXISTS entitlement_cache;
DROP TABLE IF EXISTS entitlements;
DROP TRIGGER IF EXISTS guild_settings_kv_notify ON guild_settings_kv;
DROP TRIGGER IF EXISTS guild_config_notify ON guild_config;
DROP FUNCTION IF EXISTS notify_config_changed();
DROP TABLE IF EXISTS guild_settings_kv;
ALTER TABLE guild_config DROP COLUMN IF EXISTS updated_at;
DROP TABLE IF EXISTS bot_config;
