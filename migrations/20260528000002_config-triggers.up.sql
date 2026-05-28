-- Trigger function shared by guild_config and guild_settings_kv.
-- Fires pg_notify('config_changed', <guild_id>) so the ConfigStore can
-- evict its cache entry for the affected guild (cross-process invalidation).
CREATE OR REPLACE FUNCTION notify_config_changed()
RETURNS TRIGGER AS $$
DECLARE
    guild_id_val TEXT;
BEGIN
    IF TG_OP = 'DELETE' THEN
        IF TG_TABLE_NAME = 'guild_config' THEN
            guild_id_val := OLD.id::text;
        ELSE
            guild_id_val := OLD.guild_id::text;
        END IF;
    ELSE
        IF TG_TABLE_NAME = 'guild_config' THEN
            guild_id_val := NEW.id::text;
        ELSE
            guild_id_val := NEW.guild_id::text;
        END IF;
    END IF;
    PERFORM pg_notify('config_changed', guild_id_val);
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER guild_config_notify
    AFTER INSERT OR UPDATE OR DELETE ON guild_config
    FOR EACH ROW EXECUTE FUNCTION notify_config_changed();

CREATE TRIGGER guild_settings_kv_notify
    AFTER INSERT OR UPDATE OR DELETE ON guild_settings_kv
    FOR EACH ROW EXECUTE FUNCTION notify_config_changed();
