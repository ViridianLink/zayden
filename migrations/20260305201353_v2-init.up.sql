-- Add up migration script here
CREATE TABLE users (
    id BIGINT PRIMARY KEY,
    username VARCHAR(255) NOT NULL
);

INSERT INTO users (id, username)
SELECT DISTINCT all_ids.user_id, 'PLACEHOLDER' 
FROM (
    SELECT id AS user_id FROM bingo
    UNION
    SELECT id AS user_id FROM gambling
    UNION
    SELECT id AS user_id FROM gold_stars
    UNION
    SELECT id AS user_id FROM levels
    UNION
    SELECT id AS user_id FROM lfg_users
) AS all_ids
ON CONFLICT (id) DO NOTHING;

ALTER TABLE guilds RENAME TO old_guilds;

CREATE TABLE guilds (
    id BIGINT PRIMARY KEY
);

INSERT INTO guilds (id)
SELECT DISTINCT all_ids.guild_id 
FROM (
    SELECT id AS guild_id FROM old_guilds
    UNION
    SELECT guild_id FROM reaction_roles
) AS all_ids
ON CONFLICT (id) DO NOTHING;

-- Bingo
CREATE TABLE bingo_new (
    user_id BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    day DATE NOT NULL
);

CREATE TABLE bingo_spaces (
    bingo_id BIGINT NOT NULL REFERENCES bingo_new(user_id) ON DELETE CASCADE,
    position INT NOT NULL,
    space TEXT NOT NULL,
    PRIMARY KEY (bingo_id, position)
);

INSERT INTO bingo_new (user_id, day)
SELECT id, day
FROM bingo;

INSERT INTO bingo_spaces (bingo_id, position, space)
SELECT 
    id AS bingo_id,
    gs.pos AS position,
    gs.val AS space
FROM bingo
CROSS JOIN LATERAL unnest(spaces) WITH ORDINALITY AS gs(val, pos);

DROP TABLE bingo;

ALTER TABLE bingo_new RENAME TO bingo;


-- Family
CREATE TABLE family_new (
    user_id BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE family_partners (
    user_id BIGINT NOT NULL REFERENCES family_new(user_id) ON DELETE CASCADE,
    partner_id BIGINT NOT NULL REFERENCES family_new(user_id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, partner_id),
    CHECK (user_id < partner_id)
);

CREATE TABLE family_parent_child (
    parent_id BIGINT NOT NULL REFERENCES family_new(user_id) ON DELETE CASCADE,
    child_id BIGINT NOT NULL REFERENCES family_new(user_id) ON DELETE CASCADE,
    PRIMARY KEY (parent_id, child_id),
    CHECK (parent_id <> child_id)
);

CREATE TABLE family_blocks (
    user_id BIGINT NOT NULL REFERENCES family_new(user_id) ON DELETE CASCADE,
    blocked_id BIGINT NOT NULL REFERENCES family_new(user_id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, blocked_id),
    CHECK (user_id <> blocked_id)
);

INSERT INTO family_new (user_id)
SELECT id
FROM family;

INSERT INTO family_partners (user_id, partner_id)
SELECT LEAST(p, f.id) AS user_id, GREATEST(p, f.id) AS partner_id
FROM family f
CROSS JOIN LATERAL unnest(f.partner_ids) AS p;

INSERT INTO family_parent_child (parent_id, child_id)
SELECT f.id AS parent_id, c AS child_id
FROM family f
CROSS JOIN LATERAL unnest(f.children_ids) AS c
WHERE f.id <> c;

INSERT INTO family_blocks (user_id, blocked_id)
SELECT f.id AS user_id, b AS blocked_id
FROM family f
CROSS JOIN LATERAL unnest(f.blocked_ids) AS b
WHERE f.id <> b;

DROP TABLE family;

ALTER TABLE family_new RENAME TO family;

--- Gambling
CREATE TABLE gambling_new (
    user_id BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    coins BIGINT NOT NULL DEFAULT 1000,
    daily DATE NOT NULL DEFAULT '1970-01-01',
    stamina INT NOT NULL DEFAULT 1,
    gift DATE NOT NULL DEFAULT '1970-01-01',
    gems BIGINT NOT NULL DEFAULT 0,
    CONSTRAINT coins_must_be_non_negative CHECK (coins >= 0)
);

CREATE TABLE gambling_stats_new (
    user_id BIGINT PRIMARY KEY REFERENCES gambling_new(user_id) ON DELETE CASCADE,
    max_cash BIGINT NOT NULL DEFAULT 0,
    total_cash BIGINT NOT NULL DEFAULT 0,
    gifts_given INT NOT NULL DEFAULT 0,
    gifts_received INT NOT NULL DEFAULT 0,
    higher_or_lower_score INT NOT NULL DEFAULT 0,
    weekly_higher_or_lower_score INT NOT NULL DEFAULT 0
);

CREATE TABLE gambling_inventory_new (
    id INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES gambling_new(user_id) ON DELETE CASCADE,
    item_id TEXT NOT NULL,
    quantity BIGINT NOT NULL DEFAULT 0,
    UNIQUE (user_id, item_id)
);

CREATE TABLE gambling_effects_new (
    id INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES gambling_new(user_id) ON DELETE CASCADE,
    item_id TEXT NOT NULL,
    expiry TIMESTAMPTZ,
    UNIQUE (user_id, item_id)
);

CREATE TABLE gambling_goals_new (
    id INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES gambling_new(user_id) ON DELETE CASCADE,
    goal_id TEXT NOT NULL,
    day DATE NOT NULL DEFAULT '1970-01-01',
    progress BIGINT NOT NULL DEFAULT 0,
    target BIGINT NOT NULL DEFAULT 1
);

CREATE TABLE gambling_mine_new (
    user_id BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    miners BIGINT NOT NULL DEFAULT 0,
    mines BIGINT NOT NULL DEFAULT 0,
    land BIGINT NOT NULL DEFAULT 0,
    countries BIGINT NOT NULL DEFAULT 0,
    continents BIGINT NOT NULL DEFAULT 0,
    planets BIGINT NOT NULL DEFAULT 0,
    solar_systems BIGINT NOT NULL DEFAULT 0,
    galaxies BIGINT NOT NULL DEFAULT 0,
    universes BIGINT NOT NULL DEFAULT 0,
    prestige BIGINT NOT NULL DEFAULT 0,
    coal BIGINT NOT NULL DEFAULT 0,
    iron BIGINT NOT NULL DEFAULT 0,
    gold BIGINT NOT NULL DEFAULT 0,
    redstone BIGINT NOT NULL DEFAULT 0,
    lapis BIGINT NOT NULL DEFAULT 0,
    diamonds BIGINT NOT NULL DEFAULT 0,
    emeralds BIGINT NOT NULL DEFAULT 0,
    tech BIGINT NOT NULL DEFAULT 0,
    utility BIGINT NOT NULL DEFAULT 0,
    production BIGINT NOT NULL DEFAULT 0,
    mine_activity TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO gambling_new (user_id, coins, daily, stamina, gift, gems)
SELECT id, coins, daily, stamina, gift, gems
FROM gambling;

INSERT INTO gambling_stats_new (user_id, max_cash, total_cash, gifts_given, gifts_received, higher_or_lower_score, weekly_higher_or_lower_score)
SELECT g.id, s.max_cash, s.total_cash, s.gifts_given, s.gifts_received, s.higher_or_lower_score, s.weekly_higher_or_lower_score
FROM gambling_stats s
JOIN gambling g ON g.id = s.user_id;

INSERT INTO gambling_inventory_new (user_id, item_id, quantity)
SELECT user_id, item_id, quantity
FROM gambling_inventory;

INSERT INTO gambling_effects_new (user_id, item_id, expiry)
SELECT user_id, item_id, expiry
FROM gambling_effects;

INSERT INTO gambling_goals_new (user_id, goal_id, day, progress, target)
SELECT user_id, goal_id, day, progress, target
FROM gambling_goals;

INSERT INTO gambling_mine_new (user_id, miners, mines, land, countries, continents, planets, solar_systems, galaxies, universes,
                               prestige, coal, iron, gold, redstone, lapis, diamonds, emeralds, tech, utility, production, mine_activity)
SELECT id, miners, mines, land, countries, continents, planets, solar_systems, galaxies, universes,
       prestige, coal, iron, gold, redstone, lapis, diamonds, emeralds, tech, utility, production, mine_activity
FROM gambling_mine;

DROP TABLE gambling_stats, gambling_inventory, gambling_effects, gambling_goals, gambling_mine;
DROP TABLE gambling;

ALTER TABLE gambling_new RENAME TO gambling;
ALTER TABLE gambling_stats_new RENAME TO gambling_stats;
ALTER TABLE gambling_inventory_new RENAME TO gambling_inventory;
ALTER TABLE gambling_effects_new RENAME TO gambling_effects;
ALTER TABLE gambling_goals_new RENAME TO gambling_goals;
ALTER TABLE gambling_mine_new RENAME TO gambling_mine;

CREATE INDEX idx_gambling_stats_max_cash ON gambling_stats (max_cash DESC);
CREATE INDEX idx_gambling_stats_total_cash ON gambling_stats (total_cash DESC);
CREATE INDEX idx_gambling_stats_gifts_given ON gambling_stats (gifts_given DESC);
CREATE INDEX idx_gambling_stats_gifts_received ON gambling_stats (gifts_received DESC);
CREATE INDEX idx_gambling_stats_higher_or_lower_score ON gambling_stats (higher_or_lower_score DESC);

CREATE INDEX idx_gambling_inventory_user_id ON gambling_inventory (user_id);
CREATE INDEX idx_gambling_effects_user_id ON gambling_effects (user_id);
CREATE INDEX idx_gambling_goals_user_id ON gambling_goals (user_id);

-- Gold Stars
CREATE TABLE gold_stars_new (
    id BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    number_of_stars INT NOT NULL DEFAULT 0,
    given_stars INT NOT NULL DEFAULT 0,
    received_stars INT NOT NULL DEFAULT 0,
    last_free_star TIMESTAMPTZ NOT NULL
);

INSERT INTO gold_stars_new (id, number_of_stars, given_stars, received_stars, last_free_star)
SELECT id, number_of_stars, given_stars, received_stars, last_free_star
FROM gold_stars;

DROP TABLE gold_stars;
ALTER TABLE gold_stars_new RENAME TO gold_stars;


-- Guild Config
CREATE TABLE guild_config (
    id BIGINT PRIMARY KEY REFERENCES guilds(id) ON DELETE CASCADE,
    support_channel_id BIGINT,
    support_thread_id INT NOT NULL DEFAULT 0,
    support_role_id BIGINT,
    faq_channel_id BIGINT,
    suggestions_channel_id BIGINT,
    review_channel_id BIGINT,
    rules_channel_id BIGINT,
    general_channel_id BIGINT,
    spoiler_channel_id BIGINT,
    artist_role_id BIGINT,
    sleep_role_id BIGINT,
    temp_voice_category BIGINT,
    temp_voice_creator_channel BIGINT,
    thread_id INT NOT NULL DEFAULT 0,
    lfg_channel_id BIGINT,
    lfg_role_id BIGINT,
    lfg_scheduled_thread_id BIGINT
);

CREATE TABLE guild_support_roles (
    guild_id BIGINT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    role_id BIGINT NOT NULL,
    PRIMARY KEY (guild_id, role_id)
);

CREATE TABLE guild_xp_blocked_channels (
    guild_id BIGINT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    channel_id BIGINT NOT NULL,
    PRIMARY KEY (guild_id, channel_id)
);

INSERT INTO guild_config (
    id,
    support_channel_id,
    support_thread_id,
    support_role_id,
    faq_channel_id,
    suggestions_channel_id,
    review_channel_id,
    rules_channel_id,
    general_channel_id,
    spoiler_channel_id,
    artist_role_id,
    sleep_role_id,
    temp_voice_category,
    temp_voice_creator_channel,
    thread_id,
    lfg_channel_id,
    lfg_role_id,
    lfg_scheduled_thread_id
)
SELECT
    g.id,

    COALESCE(s.support_channel_id, g.support_channel_id),
    COALESCE(s.support_thread_id, 0),
    s.support_role_id,

    g.faq_channel_id,
    COALESCE(s.suggestions_channel_id, g.suggestions_channel_id),
    g.review_channel_id,

    s.rules_channel_id,
    s.general_channel_id,
    s.spoiler_channel_id,

    s.artist_role_id,
    s.sleep_role_id,

    g.temp_voice_category,
    g.temp_voice_creator_channel,
    g.thread_id,

    l.channel_id,
    l.role_id,
    l.scheduled_thread_id

FROM old_guilds g
LEFT JOIN servers s ON s.id = g.id
LEFT JOIN lfg_guilds l ON l.id = g.id;

INSERT INTO guild_support_roles (guild_id, role_id)
SELECT
    g.id,
    r
FROM old_guilds g
CROSS JOIN LATERAL unnest(g.support_role_ids) AS r;

INSERT INTO guild_xp_blocked_channels (guild_id, channel_id)
SELECT
    g.id,
    c
FROM old_guilds g
CROSS JOIN LATERAL unnest(g.xp_blocked_channels) AS c;

DROP TABLE old_guilds;
DROP TABLE servers;
DROP TABLE lfg_guilds;


-- Moderation
CREATE TABLE infractions_new (
    id INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    username VARCHAR(255) NOT NULL,
    guild_id BIGINT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    infraction_type VARCHAR(255) NOT NULL,
    moderator_id BIGINT NOT NULL REFERENCES users(id),
    moderator_username VARCHAR(255) NOT NULL,
    points INT NOT NULL DEFAULT 1,
    reason VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 2. Copy data while preserving IDs
INSERT INTO infractions_new (
    user_id,
    username,
    guild_id,
    infraction_type,
    moderator_id,
    moderator_username,
    points,
    reason,
    created_at
)
SELECT
    user_id,
    username,
    guild_id,
    infraction_type,
    moderator_id,
    moderator_username,
    points,
    reason,
    created_at
FROM infractions;

DROP TABLE infractions;
ALTER TABLE infractions_new RENAME TO infractions;

CREATE INDEX idx_infractions_user_guild ON infractions (user_id, guild_id);


-- Levels
ALTER TABLE levels
ALTER COLUMN last_xp TYPE TIMESTAMPTZ
USING last_xp AT TIME ZONE 'UTC';

ALTER TABLE levels
ALTER COLUMN last_xp SET DEFAULT '1970-01-01 00:00:00+00';

ALTER TABLE levels
RENAME COLUMN id TO user_id;

ALTER TABLE levels
ADD CONSTRAINT levels_user_id_fkey
FOREIGN KEY (user_id)
REFERENCES users(id)
ON DELETE CASCADE;


-- LFG
ALTER TABLE lfg_users RENAME TO lfg_user_config;

ALTER TABLE lfg_user_config
ADD CONSTRAINT lfg_user_config_user_fk
FOREIGN KEY (id)
REFERENCES users(id)
ON DELETE CASCADE;

ALTER TABLE lfg_posts
RENAME COLUMN owner TO owner_id;

ALTER TABLE lfg_posts
ADD CONSTRAINT lfg_posts_owner_fk
FOREIGN KEY (owner_id)
REFERENCES users(id)
ON DELETE CASCADE;

ALTER TABLE lfg_fireteam
RENAME COLUMN post TO post_id;

ALTER TABLE lfg_fireteam
ADD CONSTRAINT lfg_fireteam_post_fk
FOREIGN KEY (post_id)
REFERENCES lfg_posts(id)
ON DELETE CASCADE;

ALTER TABLE lfg_fireteam
ADD CONSTRAINT lfg_fireteam_user_fk
FOREIGN KEY (user_id)
REFERENCES users(id)
ON DELETE CASCADE;

ALTER TABLE lfg_messages
RENAME COLUMN id TO post_id;

ALTER TABLE lfg_messages
RENAME COLUMN message TO message_id;

ALTER TABLE lfg_messages
RENAME COLUMN channel TO channel_id;

ALTER TABLE lfg_messages
ADD CONSTRAINT lfg_messages_post_fk
FOREIGN KEY (post_id)
REFERENCES lfg_posts(id)
ON DELETE CASCADE;

-- Reaction Roles
CREATE TABLE reaction_roles_new (
    id INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    guild_id BIGINT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    channel_id BIGINT NOT NULL,
    message_id BIGINT NOT NULL,
    role_id BIGINT NOT NULL,
    emoji VARCHAR(255) NOT NULL
);

INSERT INTO reaction_roles_new (
    guild_id,
    channel_id,
    message_id,
    role_id,
    emoji
)
SELECT
    guild_id,
    channel_id,
    message_id,
    role_id,
    emoji
FROM reaction_roles;

DROP TABLE reaction_roles;
ALTER TABLE reaction_roles_new RENAME TO reaction_roles;

CREATE INDEX idx_reaction_roles_guild_message
ON reaction_roles (guild_id, message_id);


-- Tickets
CREATE TABLE tickets_new (
    thread_id BIGINT PRIMARY KEY
);

CREATE TABLE ticket_roles (
    ticket_id BIGINT NOT NULL REFERENCES tickets_new(thread_id) ON DELETE CASCADE,
    role_id BIGINT NOT NULL,
    PRIMARY KEY (ticket_id, role_id)
);

INSERT INTO tickets_new (thread_id)
SELECT id
FROM tickets;

INSERT INTO ticket_roles (ticket_id, role_id)
SELECT
    t.id,
    r
FROM tickets t
CROSS JOIN LATERAL unnest(t.role_ids) AS r;

DROP TABLE tickets;
ALTER TABLE tickets_new RENAME TO tickets;


-- Voice Channels
CREATE TABLE voice_channel_trusted_users (
    channel_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    PRIMARY KEY (channel_id, user_id)
);

CREATE TABLE voice_channel_invites (
    channel_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    PRIMARY KEY (channel_id, user_id)
);

INSERT INTO voice_channel_trusted_users (channel_id, user_id)
SELECT DISTINCT vc.id, uid
FROM voice_channels vc,
LATERAL unnest(vc.trusted_ids) AS uid
JOIN users u ON u.id = uid;

INSERT INTO voice_channel_invites (channel_id, user_id)
SELECT DISTINCT vc.id, uid
FROM voice_channels vc,
LATERAL unnest(vc.invites) AS uid
JOIN users u ON u.id = uid;

ALTER TABLE voice_channels 
    DROP COLUMN trusted_ids,
    DROP COLUMN invites;


ALTER TABLE voice_channels
    ADD CONSTRAINT fk_voice_channels_owner 
    FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE voice_channel_trusted_users
    ADD CONSTRAINT fk_trusted_channel 
    FOREIGN KEY (channel_id) REFERENCES voice_channels(id) ON DELETE CASCADE,
    ADD CONSTRAINT fk_trusted_user 
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE voice_channel_invites
    ADD CONSTRAINT fk_invites_channel 
    FOREIGN KEY (channel_id) REFERENCES voice_channels(id) ON DELETE CASCADE,
    ADD CONSTRAINT fk_invites_user 
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

