DROP TRIGGER IF EXISTS entitlements_serving_notify ON entitlements;

DROP TRIGGER IF EXISTS guild_presence_serving_notify_update ON guild_presence;

DROP TRIGGER IF EXISTS guild_presence_serving_notify_insert ON guild_presence;

DROP TABLE IF EXISTS custom_bot_guilds;

DROP TABLE IF EXISTS custom_bots;

DROP FUNCTION IF EXISTS notify_serving_changed ();

DROP FUNCTION IF EXISTS notify_custom_bots_changed ();

DROP FUNCTION IF EXISTS serving_application (bigint);

DROP FUNCTION IF EXISTS custom_bot_entitled (bigint);

ALTER TABLE guild_presence
    DROP COLUMN IF EXISTS joined_at;

