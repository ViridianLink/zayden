CREATE TABLE guild_levels(
    guild_id bigint NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    user_id bigint NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    total_xp bigint NOT NULL DEFAULT 0,
    last_xp timestamptz NOT NULL DEFAULT '1970-01-01 00:00:00+00',
    xp int NOT NULL DEFAULT 0,
    level int NOT NULL DEFAULT 0,
    message_count bigint NOT NULL DEFAULT 0,
    PRIMARY KEY (guild_id, user_id)
);

CREATE INDEX guild_levels_rank_idx ON guild_levels(guild_id, level DESC, xp DESC);

