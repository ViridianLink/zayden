CREATE TABLE deadlock_watch_state (
    key text PRIMARY KEY CHECK (key IN ('latest', 'baseline')),
    client_version integer NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE deadlock_asset_snapshots (
    client_version integer PRIMARY KEY,
    heroes text NOT NULL,
    items text NOT NULL,
    captured_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE deadlock_patches (
    guid text PRIMARY KEY,
    title text NOT NULL,
    link text NOT NULL,
    published_at timestamptz NOT NULL,
    detected_at timestamptz NOT NULL DEFAULT now(),
    baseline_version integer,
    client_version integer,
    diff text,
    announced_at timestamptz,
    briefed_at timestamptz
);

CREATE INDEX deadlock_patches_unannounced ON deadlock_patches (published_at)
WHERE
    announced_at IS NULL;

CREATE TABLE deadlock_patch_announce (
    guild_id bigint PRIMARY KEY REFERENCES guilds (id) ON DELETE CASCADE,
    channel_id bigint NOT NULL,
    message text,
    ping_role_ids bigint[] NOT NULL DEFAULT '{}',
    updated_at timestamptz NOT NULL DEFAULT now()
);

