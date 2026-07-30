CREATE TABLE honeypot_settings(
    guild_id bigint PRIMARY KEY REFERENCES guilds(id) ON DELETE CASCADE,
    channel_id bigint,
    exempt_admins boolean NOT NULL DEFAULT FALSE,
    exempt_role_id bigint,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE OR REPLACE TRIGGER honeypot_settings_notify
    AFTER INSERT OR UPDATE OR DELETE ON honeypot_settings
    FOR EACH ROW
    EXECUTE FUNCTION notify_config_changed();

