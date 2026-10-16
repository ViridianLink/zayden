CREATE TABLE deadlock_api_cache (
    key text PRIMARY KEY,
    kind text NOT NULL CHECK (kind IN ('asset', 'analytics', 'build', 'patch')),
    client_version integer,
    body text NOT NULL,
    fetched_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL
);

CREATE INDEX deadlock_api_cache_expires_at ON deadlock_api_cache (expires_at);

