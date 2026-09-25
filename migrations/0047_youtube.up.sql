CREATE TABLE youtube_channels (
    channel_id text PRIMARY KEY,
    title text NOT NULL,
    uploads_playlist_id text NOT NULL,
    websub_secret text NOT NULL,
    websub_expires_at timestamptz,
    seeded_at timestamptz,
    last_polled_at timestamptz,
    consecutive_failures int NOT NULL DEFAULT 0
);

CREATE TABLE youtube_connections (
    guild_id bigint PRIMARY KEY REFERENCES guilds (id) ON DELETE CASCADE,
    channel_id text NOT NULL REFERENCES youtube_channels (channel_id) ON DELETE CASCADE,
    connected_by bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX youtube_connections_channel_idx ON youtube_connections (channel_id);

CREATE TABLE youtube_videos (
    video_id text PRIMARY KEY,
    channel_id text NOT NULL REFERENCES youtube_channels (channel_id) ON DELETE CASCADE,
    title text NOT NULL,
    published_at timestamptz NOT NULL,
    announced_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX youtube_videos_pending_idx ON youtube_videos (channel_id, published_at)
WHERE
    announced_at IS NULL;

CREATE TABLE youtube_announce (
    guild_id bigint PRIMARY KEY REFERENCES guilds (id) ON DELETE CASCADE,
    channel_id bigint NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE OR REPLACE TRIGGER youtube_announce_notify
    AFTER INSERT OR UPDATE OR DELETE ON youtube_announce
    FOR EACH ROW
    EXECUTE FUNCTION notify_config_changed ();

