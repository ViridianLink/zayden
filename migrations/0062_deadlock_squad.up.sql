ALTER TABLE deadlock_guild_members
    ADD COLUMN squad boolean NOT NULL DEFAULT FALSE;

ALTER TABLE deadlock_players
    ADD COLUMN badge integer,
    ADD COLUMN announced_streak integer NOT NULL DEFAULT 0;

CREATE TABLE deadlock_rank_snapshots (
    account_id bigint NOT NULL REFERENCES deadlock_players (account_id) ON DELETE CASCADE,
    captured_at timestamptz NOT NULL DEFAULT now(),
    badge integer NOT NULL,
    PRIMARY KEY (account_id, captured_at)
);

CREATE TABLE deadlock_squad_channel (
    guild_id bigint PRIMARY KEY REFERENCES guilds (id) ON DELETE CASCADE,
    channel_id bigint NOT NULL,
    leaderboard boolean NOT NULL DEFAULT FALSE,
    leaderboard_message_id bigint,
    updated_at timestamptz NOT NULL DEFAULT now()
);

