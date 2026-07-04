CREATE TABLE support_settings(
    guild_id bigint PRIMARY KEY REFERENCES guilds(id) ON DELETE CASCADE,
    support_channel_id bigint,
    support_thread_id int NOT NULL DEFAULT 0,
    support_role_id bigint,
    faq_channel_id bigint,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE suggestions_settings(
    guild_id bigint PRIMARY KEY REFERENCES guilds(id) ON DELETE CASCADE,
    suggestions_channel_id bigint,
    review_channel_id bigint,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE guild_channels(
    guild_id bigint PRIMARY KEY REFERENCES guilds(id) ON DELETE CASCADE,
    rules_channel_id bigint,
    general_channel_id bigint,
    spoiler_channel_id bigint,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE roles_settings(
    guild_id bigint PRIMARY KEY REFERENCES guilds(id) ON DELETE CASCADE,
    artist_role_id bigint,
    sleep_role_id bigint,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE temp_voice_settings(
    guild_id bigint PRIMARY KEY REFERENCES guilds(id) ON DELETE CASCADE,
    temp_voice_category bigint,
    temp_voice_creator_channel bigint,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE lfg_settings(
    guild_id bigint PRIMARY KEY REFERENCES guilds(id) ON DELETE CASCADE,
    lfg_channel_id bigint,
    lfg_role_id bigint,
    lfg_scheduled_thread_id bigint,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE ticket_settings(
    guild_id bigint PRIMARY KEY REFERENCES guilds(id) ON DELETE CASCADE,
    thread_id int NOT NULL DEFAULT 0,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE music_settings(
    guild_id bigint PRIMARY KEY REFERENCES guilds(id) ON DELETE CASCADE,
    dj_role_id bigint,
    default_volume smallint NOT NULL DEFAULT 100 CHECK (default_volume BETWEEN 0 AND 100),
    auto_disconnect_secs integer NOT NULL DEFAULT 120 CHECK (auto_disconnect_secs >= 0),
    stay_connected boolean NOT NULL DEFAULT FALSE,
    autoplay boolean NOT NULL DEFAULT FALSE,
    announce_now_playing boolean NOT NULL DEFAULT TRUE,
    updated_at timestamptz NOT NULL DEFAULT now()
);

-- Copy existing data out of guild_config before dropping it.
INSERT INTO support_settings(guild_id, support_channel_id, support_thread_id, support_role_id, faq_channel_id, updated_at)
SELECT
    id,
    support_channel_id,
    support_thread_id,
    support_role_id,
    faq_channel_id,
    updated_at
FROM
    guild_config;

INSERT INTO suggestions_settings(guild_id, suggestions_channel_id, review_channel_id, updated_at)
SELECT
    id,
    suggestions_channel_id,
    review_channel_id,
    updated_at
FROM
    guild_config;

INSERT INTO guild_channels(guild_id, rules_channel_id, general_channel_id, spoiler_channel_id, updated_at)
SELECT
    id,
    rules_channel_id,
    general_channel_id,
    spoiler_channel_id,
    updated_at
FROM
    guild_config;

INSERT INTO roles_settings(guild_id, artist_role_id, sleep_role_id, updated_at)
SELECT
    id,
    artist_role_id,
    sleep_role_id,
    updated_at
FROM
    guild_config;

INSERT INTO temp_voice_settings(guild_id, temp_voice_category, temp_voice_creator_channel, updated_at)
SELECT
    id,
    temp_voice_category,
    temp_voice_creator_channel,
    updated_at
FROM
    guild_config;

INSERT INTO lfg_settings(guild_id, lfg_channel_id, lfg_role_id, lfg_scheduled_thread_id, updated_at)
SELECT
    id,
    lfg_channel_id,
    lfg_role_id,
    lfg_scheduled_thread_id,
    updated_at
FROM
    guild_config;

INSERT INTO ticket_settings(guild_id, thread_id, updated_at)
SELECT
    id,
    thread_id,
    updated_at
FROM
    guild_config;

DROP TABLE guild_settings_kv;

DROP TABLE guild_config;

CREATE OR REPLACE TRIGGER support_settings_notify
    AFTER INSERT OR UPDATE OR DELETE ON support_settings
    FOR EACH ROW
    EXECUTE FUNCTION notify_config_changed();

CREATE OR REPLACE TRIGGER suggestions_settings_notify
    AFTER INSERT OR UPDATE OR DELETE ON suggestions_settings
    FOR EACH ROW
    EXECUTE FUNCTION notify_config_changed();

CREATE OR REPLACE TRIGGER guild_channels_notify
    AFTER INSERT OR UPDATE OR DELETE ON guild_channels
    FOR EACH ROW
    EXECUTE FUNCTION notify_config_changed();

CREATE OR REPLACE TRIGGER roles_settings_notify
    AFTER INSERT OR UPDATE OR DELETE ON roles_settings
    FOR EACH ROW
    EXECUTE FUNCTION notify_config_changed();

CREATE OR REPLACE TRIGGER temp_voice_settings_notify
    AFTER INSERT OR UPDATE OR DELETE ON temp_voice_settings
    FOR EACH ROW
    EXECUTE FUNCTION notify_config_changed();

CREATE OR REPLACE TRIGGER lfg_settings_notify
    AFTER INSERT OR UPDATE OR DELETE ON lfg_settings
    FOR EACH ROW
    EXECUTE FUNCTION notify_config_changed();

CREATE OR REPLACE TRIGGER ticket_settings_notify
    AFTER INSERT OR UPDATE OR DELETE ON ticket_settings
    FOR EACH ROW
    EXECUTE FUNCTION notify_config_changed();

CREATE OR REPLACE TRIGGER music_settings_notify
    AFTER INSERT OR UPDATE OR DELETE ON music_settings
    FOR EACH ROW
    EXECUTE FUNCTION notify_config_changed();

