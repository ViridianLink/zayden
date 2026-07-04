CREATE TABLE guild_config(
    id bigint PRIMARY KEY REFERENCES guilds(id) ON DELETE CASCADE,
    support_channel_id bigint,
    support_thread_id int NOT NULL DEFAULT 0,
    support_role_id bigint,
    faq_channel_id bigint,
    suggestions_channel_id bigint,
    review_channel_id bigint,
    rules_channel_id bigint,
    general_channel_id bigint,
    spoiler_channel_id bigint,
    artist_role_id bigint,
    sleep_role_id bigint,
    temp_voice_category bigint,
    temp_voice_creator_channel bigint,
    thread_id int NOT NULL DEFAULT 0,
    lfg_channel_id bigint,
    lfg_role_id bigint,
    lfg_scheduled_thread_id bigint,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE guild_settings_kv(
    guild_id bigint NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    module text NOT NULL,
    key TEXT NOT NULL,
    value jsonb NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (guild_id, module, key)
);

INSERT INTO guild_config(id)
SELECT
    guild_id
FROM
    support_settings
UNION
SELECT
    guild_id
FROM
    suggestions_settings
UNION
SELECT
    guild_id
FROM
    guild_channels
UNION
SELECT
    guild_id
FROM
    roles_settings
UNION
SELECT
    guild_id
FROM
    temp_voice_settings
UNION
SELECT
    guild_id
FROM
    lfg_settings
UNION
SELECT
    guild_id
FROM
    ticket_settings;

UPDATE
    guild_config gc
SET
    support_channel_id = s.support_channel_id,
    support_thread_id = s.support_thread_id,
    support_role_id = s.support_role_id,
    faq_channel_id = s.faq_channel_id
FROM
    support_settings s
WHERE
    s.guild_id = gc.id;

UPDATE
    guild_config gc
SET
    suggestions_channel_id = s.suggestions_channel_id,
    review_channel_id = s.review_channel_id
FROM
    suggestions_settings s
WHERE
    s.guild_id = gc.id;

UPDATE
    guild_config gc
SET
    rules_channel_id = c.rules_channel_id,
    general_channel_id = c.general_channel_id,
    spoiler_channel_id = c.spoiler_channel_id
FROM
    guild_channels c
WHERE
    c.guild_id = gc.id;

UPDATE
    guild_config gc
SET
    artist_role_id = r.artist_role_id,
    sleep_role_id = r.sleep_role_id
FROM
    roles_settings r
WHERE
    r.guild_id = gc.id;

UPDATE
    guild_config gc
SET
    temp_voice_category = t.temp_voice_category,
    temp_voice_creator_channel = t.temp_voice_creator_channel
FROM
    temp_voice_settings t
WHERE
    t.guild_id = gc.id;

UPDATE
    guild_config gc
SET
    lfg_channel_id = l.lfg_channel_id,
    lfg_role_id = l.lfg_role_id,
    lfg_scheduled_thread_id = l.lfg_scheduled_thread_id
FROM
    lfg_settings l
WHERE
    l.guild_id = gc.id;

UPDATE
    guild_config gc
SET
    thread_id = tk.thread_id
FROM
    ticket_settings tk
WHERE
    tk.guild_id = gc.id;

CREATE OR REPLACE TRIGGER guild_config_notify
    AFTER INSERT OR UPDATE OR DELETE ON guild_config
    FOR EACH ROW
    EXECUTE FUNCTION notify_config_changed();

CREATE OR REPLACE TRIGGER guild_settings_kv_notify
    AFTER INSERT OR UPDATE OR DELETE ON guild_settings_kv
    FOR EACH ROW
    EXECUTE FUNCTION notify_config_changed();

DROP TABLE music_settings;

DROP TABLE ticket_settings;

DROP TABLE lfg_settings;

DROP TABLE temp_voice_settings;

DROP TABLE roles_settings;

DROP TABLE guild_channels;

DROP TABLE suggestions_settings;

DROP TABLE support_settings;

