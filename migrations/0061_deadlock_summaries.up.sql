CREATE TABLE deadlock_user_settings (
    discord_id bigint PRIMARY KEY,
    dm_summaries boolean NOT NULL DEFAULT TRUE,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE deadlock_guild_members (
    guild_id bigint NOT NULL REFERENCES guilds (id) ON DELETE CASCADE,
    discord_id bigint NOT NULL,
    share_summaries boolean NOT NULL DEFAULT FALSE,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (guild_id, discord_id)
);

CREATE TABLE deadlock_summary_channel (
    guild_id bigint PRIMARY KEY REFERENCES guilds (id) ON DELETE CASCADE,
    channel_id bigint NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE deadlock_players (
    account_id bigint PRIMARY KEY,
    discord_id bigint NOT NULL UNIQUE,
    provider text NOT NULL DEFAULT 'steam' CHECK (provider = 'steam'),
    seeded_at timestamptz NOT NULL DEFAULT now(),
    streak integer NOT NULL DEFAULT 0,
    last_polled_at timestamptz,
    FOREIGN KEY (discord_id, provider) REFERENCES account_links (discord_id, provider) ON DELETE CASCADE
);

CREATE TABLE deadlock_matches_seen (
    account_id bigint NOT NULL REFERENCES deadlock_players (account_id) ON DELETE CASCADE,
    match_id bigint NOT NULL,
    hero_id integer NOT NULL,
    won boolean NOT NULL,
    start_time timestamptz NOT NULL,
    processed_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, match_id)
);

CREATE INDEX deadlock_matches_seen_recent ON deadlock_matches_seen (account_id, start_time DESC);

CREATE TABLE deadlock_guild_posts (
    guild_id bigint NOT NULL REFERENCES guilds (id) ON DELETE CASCADE,
    match_id bigint NOT NULL,
    posted_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (guild_id, match_id)
);

