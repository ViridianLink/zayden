-- Stores one row per active subscription from any provider.
CREATE TABLE entitlements (
    id BIGSERIAL PRIMARY KEY,
    provider TEXT NOT NULL,
    external_id TEXT NOT NULL,
    scope_type TEXT NOT NULL CHECK (scope_type IN ('user', 'guild', 'user_in_guild')),
    scope_id BIGINT NOT NULL,
    scope_secondary_id BIGINT NOT NULL DEFAULT 0,
    tier TEXT NOT NULL CHECK (tier IN ('free', 'pro', 'enterprise')),
    granted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ,
    UNIQUE (provider, external_id)
);

-- Denormalised, per-scope maximum tier, kept fresh by writes to `entitlements`.
CREATE TABLE entitlement_cache (
    scope_type TEXT NOT NULL,
    scope_id BIGINT NOT NULL,
    scope_secondary_id BIGINT NOT NULL DEFAULT 0,
    tier TEXT NOT NULL,
    refreshed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (scope_type, scope_id, scope_secondary_id)
);
