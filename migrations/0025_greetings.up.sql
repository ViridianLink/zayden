CREATE TABLE greetings_settings (
    guild_id bigint PRIMARY KEY REFERENCES guilds (id) ON DELETE CASCADE,
    morning_message text,
    night_message text,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE OR REPLACE TRIGGER greetings_settings_notify
    AFTER INSERT OR UPDATE OR DELETE ON greetings_settings
    FOR EACH ROW
    EXECUTE FUNCTION notify_config_changed ();

CREATE TABLE greeting_images (
    id int GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    guild_id bigint NOT NULL REFERENCES guilds (id) ON DELETE CASCADE,
    kind text NOT NULL CONSTRAINT greeting_images_kind_valid CHECK (kind IN ('morning', 'night')),
    url text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (guild_id, kind, url)
);

CREATE INDEX idx_greeting_images_guild_kind ON greeting_images (guild_id, kind);

