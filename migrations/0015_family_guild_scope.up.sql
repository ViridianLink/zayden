DROP TABLE IF EXISTS family_blocks;

DROP TABLE IF EXISTS family_parent_child;

DROP TABLE IF EXISTS family_partners;

DROP TABLE IF EXISTS family;

CREATE TABLE family (
    guild_id bigint NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    user_id bigint NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    PRIMARY KEY (guild_id, user_id)
);

CREATE TABLE family_partners(
    guild_id bigint NOT NULL,
    user_id bigint NOT NULL,
    partner_id bigint NOT NULL,
    PRIMARY KEY (guild_id, user_id, partner_id),
    FOREIGN KEY (guild_id, user_id) REFERENCES family(guild_id, user_id) ON DELETE CASCADE,
    FOREIGN KEY (guild_id, partner_id) REFERENCES family(guild_id, user_id) ON DELETE CASCADE,
    CHECK (user_id < partner_id)
);

CREATE TABLE family_parent_child(
    guild_id bigint NOT NULL,
    parent_id bigint NOT NULL,
    child_id bigint NOT NULL,
    PRIMARY KEY (guild_id, parent_id, child_id),
    FOREIGN KEY (guild_id, parent_id) REFERENCES family(guild_id, user_id) ON DELETE CASCADE,
    FOREIGN KEY (guild_id, child_id) REFERENCES family(guild_id, user_id) ON DELETE CASCADE,
    CHECK (parent_id <> child_id)
);

CREATE TABLE family_blocks(
    guild_id bigint NOT NULL,
    user_id bigint NOT NULL,
    blocked_id bigint NOT NULL,
    PRIMARY KEY (guild_id, user_id, blocked_id),
    FOREIGN KEY (guild_id, user_id) REFERENCES family(guild_id, user_id) ON DELETE CASCADE,
    FOREIGN KEY (guild_id, blocked_id) REFERENCES family(guild_id, user_id) ON DELETE CASCADE,
    CHECK (user_id <> blocked_id)
);

CREATE TABLE family_settings(
    guild_id bigint PRIMARY KEY REFERENCES guilds(id) ON DELETE CASCADE,
    max_partners int NOT NULL DEFAULT 1,
    updated_at timestamptz NOT NULL DEFAULT now()
);

