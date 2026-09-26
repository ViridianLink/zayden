CREATE TABLE guild_presence (
    guild_id bigint NOT NULL REFERENCES guilds (id) ON DELETE CASCADE,
    application_id bigint NOT NULL,
    left_at timestamptz,
    PRIMARY KEY (guild_id, application_id)
);

CREATE INDEX guild_presence_left_idx ON guild_presence (left_at)
WHERE
    left_at IS NOT NULL;

INSERT INTO guilds (id)
SELECT DISTINCT
    guild_id
FROM
    guild_rules
ON CONFLICT (id)
    DO NOTHING;

ALTER TABLE guild_rules
    ADD CONSTRAINT guild_rules_guild_id_fkey FOREIGN KEY (guild_id) REFERENCES guilds (id) ON DELETE CASCADE;

