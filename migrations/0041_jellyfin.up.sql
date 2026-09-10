CREATE TABLE jellyfin_users (
    user_id bigint PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    jellyfin_user_id text NOT NULL UNIQUE,
    jellyfin_username text NOT NULL,
    jellyseerr_user_id int,
    jellyseerr_checked_at timestamptz,
    streak_public boolean NOT NULL DEFAULT FALSE,
    letterboxd_username text,
    linked_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE jellyfin_settings (
    guild_id bigint PRIMARY KEY REFERENCES guilds (id) ON DELETE CASCADE,
    party_channel_id bigint,
    game_channel_id bigint,
    guests_enabled boolean NOT NULL DEFAULT FALSE,
    max_concurrent_guests int NOT NULL DEFAULT 20,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE OR REPLACE TRIGGER jellyfin_settings_notify
    AFTER INSERT OR UPDATE OR DELETE ON jellyfin_settings
    FOR EACH ROW
    EXECUTE FUNCTION notify_config_changed ();

CREATE TABLE jellyfin_library_items (
    item_id text PRIMARY KEY,
    item_type text NOT NULL,
    name text NOT NULL,
    sort_name text NOT NULL,
    production_year int,
    tmdb_id int,
    imdb_id text,
    tvdb_id int,
    runtime_ticks bigint,
    community_rating real,
    genres text[] NOT NULL DEFAULT '{}',
    parent_path text,
    date_created timestamptz,
    refreshed_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX jellyfin_library_items_tmdb_idx ON jellyfin_library_items (item_type, tmdb_id)
WHERE
    tmdb_id IS NOT NULL;

CREATE INDEX jellyfin_library_items_imdb_idx ON jellyfin_library_items (imdb_id)
WHERE
    imdb_id IS NOT NULL;

CREATE INDEX jellyfin_library_items_sort_idx ON jellyfin_library_items (item_type, sort_name);

CREATE TABLE jellyfin_playback_daily (
    jellyfin_user_id text NOT NULL,
    day date NOT NULL,
    seconds bigint NOT NULL,
    items int NOT NULL,
    PRIMARY KEY (jellyfin_user_id, day)
);

CREATE TABLE jellyfin_parties (
    id bigserial PRIMARY KEY,
    guild_id bigint NOT NULL REFERENCES guilds (id) ON DELETE CASCADE,
    channel_id bigint NOT NULL,
    message_id bigint,
    event_id bigint,
    host_id bigint NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    item_id text NOT NULL,
    item_name text NOT NULL,
    item_type text NOT NULL,
    item_parent_path text NOT NULL,
    guests_enabled boolean NOT NULL DEFAULT TRUE,
    library_name text UNIQUE,
    library_item_id text,
    starts_at timestamptz NOT NULL,
    ends_at timestamptz NOT NULL,
    cleanup_after timestamptz NOT NULL,
    state text NOT NULL DEFAULT 'scheduled',
    provisioned_at timestamptz,
    cleaned_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX jellyfin_parties_due_idx ON jellyfin_parties (cleanup_after)
WHERE
    cleaned_at IS NULL AND state <> 'cancelled';

CREATE INDEX jellyfin_parties_guild_idx ON jellyfin_parties (guild_id, starts_at)
WHERE
    state IN ('scheduled', 'provisioned', 'running');

CREATE TABLE jellyfin_party_guests (
    party_id bigint NOT NULL REFERENCES jellyfin_parties (id) ON DELETE CASCADE,
    user_id bigint NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    jellyfin_username text NOT NULL,
    jellyfin_user_id text,
    provisioned_at timestamptz,
    deleted_at timestamptz,
    joined_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (party_id, user_id)
);

CREATE INDEX jellyfin_party_guests_live_idx ON jellyfin_party_guests (party_id)
WHERE
    jellyfin_user_id IS NOT NULL AND deleted_at IS NULL;

CREATE TABLE jellyfin_game_rounds (
    id bigserial PRIMARY KEY,
    guild_id bigint NOT NULL REFERENCES guilds (id) ON DELETE CASCADE,
    channel_id bigint NOT NULL,
    message_id bigint,
    game text NOT NULL,
    kind text NOT NULL,
    answer text NOT NULL,
    choices jsonb,
    started_by bigint NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    solved_by bigint REFERENCES users (id) ON DELETE SET NULL,
    solved_at timestamptz,
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX jellyfin_game_rounds_open_idx ON jellyfin_game_rounds (expires_at)
WHERE
    solved_at IS NULL;

CREATE TABLE jellyfin_game_scores (
    guild_id bigint NOT NULL REFERENCES guilds (id) ON DELETE CASCADE,
    user_id bigint NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    game text NOT NULL,
    points int NOT NULL DEFAULT 0,
    correct int NOT NULL DEFAULT 0,
    played int NOT NULL DEFAULT 0,
    last_played_at timestamptz,
    PRIMARY KEY (guild_id, user_id, game)
);

CREATE INDEX jellyfin_game_scores_board_idx ON jellyfin_game_scores (guild_id, game, points DESC);

