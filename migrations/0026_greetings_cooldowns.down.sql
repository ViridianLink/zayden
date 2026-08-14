ALTER TABLE greetings_settings
    DROP COLUMN IF EXISTS user_cooldown_secs,
    DROP COLUMN IF EXISTS guild_cooldown_secs;

