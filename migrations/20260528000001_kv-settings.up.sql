-- Add updated_at tracking to guild_config
ALTER TABLE guild_config
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

-- Per-module extensible settings; new modules never require ALTER TABLE
CREATE TABLE guild_settings_kv (
    guild_id   BIGINT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    module     TEXT NOT NULL,
    key        TEXT NOT NULL,
    value      JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (guild_id, module, key)
);
