ALTER TABLE guilds
    ADD COLUMN bot_joined_at timestamptz;

CREATE TABLE guild_modules (
    guild_id bigint NOT NULL REFERENCES guilds (id) ON DELETE CASCADE,
    module text NOT NULL,
    enabled boolean NOT NULL,
    PRIMARY KEY (guild_id, module)
);

