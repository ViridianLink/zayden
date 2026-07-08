CREATE TABLE marathon_announce(
    guild_id bigint PRIMARY KEY REFERENCES guilds(id) ON DELETE CASCADE,
    channel_id bigint NOT NULL,
    last_rotation text,
    created_at timestamptz NOT NULL DEFAULT now()
);

