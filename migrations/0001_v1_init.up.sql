CREATE TYPE temp_voice_mode AS ENUM (
    'open',
    'spectator',
    'locked',
    'invisible'
);

CREATE TABLE users (
    id       BIGINT       PRIMARY KEY,
    username VARCHAR(255) NOT NULL
);

CREATE TABLE bot_tokens (
    token TEXT PRIMARY KEY,
    name  TEXT NOT NULL
);

CREATE TABLE bingo (
    user_id BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    day     DATE   NOT NULL
);

CREATE TABLE bingo_spaces (
    bingo_id BIGINT NOT NULL REFERENCES bingo(user_id) ON DELETE CASCADE,
    position INT    NOT NULL,
    space    TEXT   NOT NULL,
    PRIMARY KEY (bingo_id, position)
);

CREATE TABLE family (
    user_id BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE family_partners (
    user_id    BIGINT NOT NULL REFERENCES family(user_id) ON DELETE CASCADE,
    partner_id BIGINT NOT NULL REFERENCES family(user_id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, partner_id),
    CHECK (user_id < partner_id)
);

CREATE TABLE family_parent_child (
    parent_id BIGINT NOT NULL REFERENCES family(user_id) ON DELETE CASCADE,
    child_id  BIGINT NOT NULL REFERENCES family(user_id) ON DELETE CASCADE,
    PRIMARY KEY (parent_id, child_id),
    CHECK (parent_id <> child_id)
);

CREATE TABLE family_blocks (
    user_id    BIGINT NOT NULL REFERENCES family(user_id) ON DELETE CASCADE,
    blocked_id BIGINT NOT NULL REFERENCES family(user_id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, blocked_id),
    CHECK (user_id <> blocked_id)
);

CREATE TABLE gambling (
    user_id BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    coins   BIGINT NOT NULL DEFAULT 1000,
    daily   DATE   NOT NULL DEFAULT '1970-01-01',
    stamina INT    NOT NULL DEFAULT 1,
    gift    DATE   NOT NULL DEFAULT '1970-01-01',
    gems    BIGINT NOT NULL DEFAULT 0,
    CONSTRAINT coins_must_be_non_negative CHECK (coins >= 0)
);

CREATE TABLE gambling_stats (
    user_id                      BIGINT PRIMARY KEY REFERENCES gambling(user_id) ON DELETE CASCADE,
    max_cash                     BIGINT NOT NULL DEFAULT 0,
    total_cash                   BIGINT NOT NULL DEFAULT 0,
    gifts_given                  INT    NOT NULL DEFAULT 0,
    gifts_received               INT    NOT NULL DEFAULT 0,
    higher_or_lower_score        INT    NOT NULL DEFAULT 0,
    weekly_higher_or_lower_score INT    NOT NULL DEFAULT 0
);

CREATE TABLE gambling_inventory (
    id       INT    GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id  BIGINT NOT NULL REFERENCES gambling(user_id) ON DELETE CASCADE,
    item_id  TEXT   NOT NULL,
    quantity BIGINT NOT NULL DEFAULT 0,
    UNIQUE (user_id, item_id)
);

CREATE TABLE gambling_effects (
    id      INT    GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES gambling(user_id) ON DELETE CASCADE,
    item_id TEXT   NOT NULL,
    expiry  TIMESTAMPTZ,
    UNIQUE (user_id, item_id)
);

CREATE TABLE gambling_goals (
    id       INT    GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id  BIGINT NOT NULL REFERENCES gambling(user_id) ON DELETE CASCADE,
    goal_id  TEXT   NOT NULL,
    day      DATE   NOT NULL DEFAULT '1970-01-01',
    progress BIGINT NOT NULL DEFAULT 0,
    target   BIGINT NOT NULL DEFAULT 1
);

CREATE TABLE gambling_mine (
    user_id       BIGINT      PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    miners        BIGINT      NOT NULL DEFAULT 0,
    mines         BIGINT      NOT NULL DEFAULT 0,
    land          BIGINT      NOT NULL DEFAULT 0,
    countries     BIGINT      NOT NULL DEFAULT 0,
    continents    BIGINT      NOT NULL DEFAULT 0,
    planets       BIGINT      NOT NULL DEFAULT 0,
    solar_systems BIGINT      NOT NULL DEFAULT 0,
    galaxies      BIGINT      NOT NULL DEFAULT 0,
    universes     BIGINT      NOT NULL DEFAULT 0,
    prestige      BIGINT      NOT NULL DEFAULT 0,
    coal          BIGINT      NOT NULL DEFAULT 0,
    iron          BIGINT      NOT NULL DEFAULT 0,
    gold          BIGINT      NOT NULL DEFAULT 0,
    redstone      BIGINT      NOT NULL DEFAULT 0,
    lapis         BIGINT      NOT NULL DEFAULT 0,
    diamonds      BIGINT      NOT NULL DEFAULT 0,
    emeralds      BIGINT      NOT NULL DEFAULT 0,
    tech          BIGINT      NOT NULL DEFAULT 0,
    utility       BIGINT      NOT NULL DEFAULT 0,
    production    BIGINT      NOT NULL DEFAULT 0,
    mine_activity TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE gold_stars (
    id              BIGINT      PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    number_of_stars INT         NOT NULL DEFAULT 0,
    given_stars     INT         NOT NULL DEFAULT 0,
    received_stars  INT         NOT NULL DEFAULT 0,
    last_free_star  TIMESTAMPTZ NOT NULL
);

CREATE TABLE guilds (
    id BIGINT PRIMARY KEY
);

CREATE TABLE guild_config (
    id                         BIGINT PRIMARY KEY REFERENCES guilds(id) ON DELETE CASCADE,
    support_channel_id         BIGINT,
    support_thread_id          INT NOT NULL DEFAULT 0,
    support_role_id            BIGINT,
    faq_channel_id             BIGINT,
    suggestions_channel_id     BIGINT,
    review_channel_id          BIGINT,
    rules_channel_id           BIGINT,
    general_channel_id         BIGINT,
    spoiler_channel_id         BIGINT,
    artist_role_id             BIGINT,
    sleep_role_id              BIGINT,
    temp_voice_category        BIGINT,
    temp_voice_creator_channel BIGINT,
    thread_id                  INT NOT NULL DEFAULT 0,
    lfg_channel_id             BIGINT,
    lfg_role_id                BIGINT,
    lfg_scheduled_thread_id    BIGINT
);

CREATE TABLE guild_support_roles (
    guild_id BIGINT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    role_id  BIGINT NOT NULL,
    PRIMARY KEY (guild_id, role_id)
);

CREATE TABLE guild_xp_blocked_channels (
    guild_id   BIGINT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    channel_id BIGINT NOT NULL,
    PRIMARY KEY (guild_id, channel_id)
);

CREATE TABLE infractions (
    id                 INT          GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id            BIGINT       NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    username           VARCHAR(255) NOT NULL,
    guild_id           BIGINT       NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    infraction_type    VARCHAR(255) NOT NULL,
    moderator_id       BIGINT       NOT NULL REFERENCES users(id),
    moderator_username VARCHAR(255) NOT NULL,
    points             INT          NOT NULL DEFAULT 1,
    reason             VARCHAR(255) NOT NULL,
    created_at         TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE TABLE levels (
    user_id       BIGINT      PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    total_xp      INT         NOT NULL DEFAULT 0,
    last_xp       TIMESTAMPTZ NOT NULL DEFAULT '1970-01-01 00:00:00+00',
    xp            INT         NOT NULL DEFAULT 0,
    level         INT         NOT NULL DEFAULT 0,
    message_count INT         NOT NULL DEFAULT 0
);

CREATE TABLE lfg_user_config (
    id       BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    timezone TEXT   NOT NULL
);

CREATE TABLE lfg_posts (
    id            BIGINT      PRIMARY KEY,
    owner_id      BIGINT      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    activity      TEXT        NOT NULL,
    start_time    TIMESTAMPTZ NOT NULL,
    description   TEXT        NOT NULL DEFAULT '',
    fireteam_size SMALLINT    NOT NULL
);

CREATE TABLE lfg_fireteam (
    post_id     BIGINT  NOT NULL REFERENCES lfg_posts(id) ON DELETE CASCADE,
    user_id     BIGINT  NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    alternative BOOLEAN NOT NULL DEFAULT false,
    PRIMARY KEY (post_id, user_id)
);

CREATE TABLE lfg_messages (
    post_id    BIGINT NOT NULL PRIMARY KEY REFERENCES lfg_posts(id) ON DELETE CASCADE,
    message_id BIGINT NOT NULL,
    channel_id BIGINT NOT NULL
);

CREATE TABLE reaction_roles (
    id         INT          GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    guild_id   BIGINT       NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    channel_id BIGINT       NOT NULL,
    message_id BIGINT       NOT NULL,
    role_id    BIGINT       NOT NULL,
    emoji      VARCHAR(255) NOT NULL
);

CREATE TABLE tickets (
    thread_id BIGINT PRIMARY KEY
);

CREATE TABLE ticket_roles (
    ticket_id BIGINT NOT NULL REFERENCES tickets(thread_id) ON DELETE CASCADE,
    role_id   BIGINT NOT NULL,
    PRIMARY KEY (ticket_id, role_id)
);

CREATE TABLE voice_channels (
    id         BIGINT          PRIMARY KEY,
    owner_id   BIGINT          NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    password   TEXT,
    persistent BOOLEAN         NOT NULL DEFAULT false,
    mode       temp_voice_mode NOT NULL DEFAULT 'open'
);

CREATE TABLE voice_channel_trusted_users (
    channel_id BIGINT NOT NULL REFERENCES voice_channels(id) ON DELETE CASCADE,
    user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    PRIMARY KEY (channel_id, user_id)
);

CREATE TABLE voice_channel_invites (
    channel_id BIGINT NOT NULL REFERENCES voice_channels(id) ON DELETE CASCADE,
    user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    PRIMARY KEY (channel_id, user_id)
);

-- Indexes
CREATE INDEX idx_gambling_stats_max_cash ON gambling_stats (max_cash DESC);
CREATE INDEX idx_gambling_stats_total_cash ON gambling_stats (total_cash DESC);
CREATE INDEX idx_gambling_stats_gifts_given ON gambling_stats (gifts_given DESC);
CREATE INDEX idx_gambling_stats_gifts_received ON gambling_stats (gifts_received DESC);
CREATE INDEX idx_gambling_stats_higher_or_lower_score ON gambling_stats (higher_or_lower_score DESC);
CREATE INDEX idx_gambling_inventory_user_id ON gambling_inventory (user_id);
CREATE INDEX idx_gambling_effects_user_id ON gambling_effects (user_id);
CREATE INDEX idx_gambling_goals_user_id ON gambling_goals (user_id);
CREATE INDEX idx_lfg_posts_owner_id ON lfg_posts (owner_id);
CREATE INDEX idx_infractions_user_guild ON infractions (user_id, guild_id);
CREATE INDEX idx_reaction_roles_guild_message ON reaction_roles (guild_id, message_id);
