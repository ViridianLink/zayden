CREATE TABLE ai_settings (
    guild_id bigint PRIMARY KEY REFERENCES guilds (id) ON DELETE CASCADE,
    enabled boolean NOT NULL DEFAULT FALSE,
    channel_id bigint,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE OR REPLACE TRIGGER ai_settings_notify
    AFTER INSERT OR UPDATE OR DELETE ON ai_settings
    FOR EACH ROW
    EXECUTE FUNCTION notify_config_changed ();

