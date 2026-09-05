CREATE TABLE patreon_oauth (
    guild_id bigint PRIMARY KEY REFERENCES guilds (id) ON DELETE CASCADE,
    campaign_id text NOT NULL,
    creator_name text,
    access_token text NOT NULL,
    refresh_token text NOT NULL,
    expires_at timestamptz NOT NULL,
    webhook_id text,
    webhook_secret text,
    disabled_at timestamptz,
    connected_by bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX patreon_oauth_campaign_idx ON patreon_oauth (campaign_id);

CREATE TABLE patreon_campaigns (
    campaign_id text PRIMARY KEY,
    next_cursor text,
    seeded_at timestamptz,
    last_polled_at timestamptz,
    consecutive_failures int NOT NULL DEFAULT 0
);

CREATE TABLE patreon_posts (
    post_id text PRIMARY KEY,
    campaign_id text NOT NULL REFERENCES patreon_campaigns (campaign_id) ON DELETE CASCADE,
    title text,
    url text NOT NULL,
    content_html text,
    thumbnail_url text,
    is_public boolean NOT NULL DEFAULT FALSE,
    published_at timestamptz NOT NULL,
    announced_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX patreon_posts_pending_idx ON patreon_posts (campaign_id, published_at)
WHERE
    announced_at IS NULL;

CREATE TABLE patreon_announce (
    guild_id bigint PRIMARY KEY REFERENCES guilds (id) ON DELETE CASCADE,
    channel_id bigint NOT NULL,
    public_only boolean NOT NULL DEFAULT FALSE,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE OR REPLACE TRIGGER patreon_announce_notify
    AFTER INSERT OR UPDATE OR DELETE ON patreon_announce
    FOR EACH ROW
    EXECUTE FUNCTION notify_config_changed ();

