CREATE TABLE guild_rules(
    guild_id bigint PRIMARY KEY,
    channel_id bigint,
    message_id bigint,
    title text NOT NULL DEFAULT 'Server Rules',
    description text,
    colour integer NOT NULL DEFAULT 16711680 -- 0xFF0000
);

CREATE TABLE guild_rule(
    id integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    guild_id bigint NOT NULL REFERENCES guild_rules(guild_id) ON DELETE CASCADE,
    position integer NOT NULL,
    title text NOT NULL,
    body text NOT NULL
);

CREATE INDEX guild_rule_guild_position_idx ON guild_rule(guild_id, position);

