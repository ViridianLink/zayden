ALTER TABLE guild_rules
    DROP CONSTRAINT IF EXISTS guild_rules_guild_id_fkey;

DROP TABLE IF EXISTS guild_presence;

