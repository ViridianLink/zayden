ALTER TABLE greetings_settings
    ADD COLUMN user_cooldown_secs integer NOT NULL DEFAULT 15 CONSTRAINT greetings_settings_user_cooldown_range CHECK (user_cooldown_secs BETWEEN 0 AND 86400),
    ADD COLUMN guild_cooldown_secs integer NOT NULL DEFAULT 3 CONSTRAINT greetings_settings_guild_cooldown_range CHECK (guild_cooldown_secs BETWEEN 0 AND 86400);
