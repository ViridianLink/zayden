-- Schema additions introduced in milestones M1–M4.
-- Applied on top of 0001_v1_init.

-- M1.2: deployment-level runtime overrides (single-row, enforced by CHECK).
-- Insert a row to activate overrides: INSERT INTO bot_config DEFAULT VALUES;
CREATE TABLE bot_config (
    id                 SMALLINT PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    error_log_webhook  TEXT,
    normal_log_webhook TEXT,
    feature_flags      JSONB NOT NULL DEFAULT '{}'
);

-- M2.1: track last-modified time on every guild config row.
ALTER TABLE guild_config
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

-- M2.1: per-module extensible key-value settings; new modules never require ALTER TABLE.
CREATE TABLE guild_settings_kv (
    guild_id   BIGINT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    module     TEXT   NOT NULL,
    key        TEXT   NOT NULL,
    value      JSONB  NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (guild_id, module, key)
);

-- M2.1: LISTEN/NOTIFY trigger so ConfigStore can evict its in-process cache
-- when another process (web backend) writes a guild config change.
CREATE OR REPLACE FUNCTION notify_config_changed()
RETURNS TRIGGER AS $$
DECLARE
    guild_id_val TEXT;
BEGIN
    IF TG_OP = 'DELETE' THEN
        IF TG_TABLE_NAME = 'guild_config' THEN
            guild_id_val := OLD.id::text;
        ELSE
            guild_id_val := OLD.guild_id::text;
        END IF;
    ELSE
        IF TG_TABLE_NAME = 'guild_config' THEN
            guild_id_val := NEW.id::text;
        ELSE
            guild_id_val := NEW.guild_id::text;
        END IF;
    END IF;
    PERFORM pg_notify('config_changed', guild_id_val);
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER guild_config_notify
    AFTER INSERT OR UPDATE OR DELETE ON guild_config
    FOR EACH ROW EXECUTE FUNCTION notify_config_changed();

CREATE TRIGGER guild_settings_kv_notify
    AFTER INSERT OR UPDATE OR DELETE ON guild_settings_kv
    FOR EACH ROW EXECUTE FUNCTION notify_config_changed();

-- M4.1: one row per active subscription from any provider.
CREATE TABLE entitlements (
    id                 BIGSERIAL   PRIMARY KEY,
    provider           TEXT        NOT NULL,
    external_id        TEXT        NOT NULL,
    scope_type         TEXT        NOT NULL CHECK (scope_type IN ('user', 'guild', 'user_in_guild')),
    scope_id           BIGINT      NOT NULL,
    scope_secondary_id BIGINT      NOT NULL DEFAULT 0,
    tier               TEXT        NOT NULL CHECK (tier IN ('free', 'pro', 'enterprise')),
    granted_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at         TIMESTAMPTZ,
    UNIQUE (provider, external_id)
);

-- M10.1: LISTEN/NOTIFY trigger so EntitlementService can evict its in-process cache
-- when another process writes an entitlement change. Fires on `entitlements` only
-- (not `entitlement_cache`) to prevent double-fire.
CREATE OR REPLACE FUNCTION notify_entitlement_changed()
RETURNS TRIGGER AS $$
DECLARE
    scope_type_val         TEXT;
    scope_id_val           TEXT;
    scope_secondary_id_val TEXT;
BEGIN
    IF TG_OP = 'DELETE' THEN
        scope_type_val         := OLD.scope_type;
        scope_id_val           := OLD.scope_id::text;
        scope_secondary_id_val := COALESCE(OLD.scope_secondary_id::text, '');
    ELSE
        scope_type_val         := NEW.scope_type;
        scope_id_val           := NEW.scope_id::text;
        scope_secondary_id_val := COALESCE(NEW.scope_secondary_id::text, '');
    END IF;
    PERFORM pg_notify('entitlement_changed', scope_type_val || ':' || scope_id_val || ':' || scope_secondary_id_val);
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER notify_entitlement_changed
    AFTER INSERT OR UPDATE OR DELETE ON entitlements
    FOR EACH ROW EXECUTE FUNCTION notify_entitlement_changed();

-- M4.1: denormalised per-scope maximum tier, kept fresh by writes to `entitlements`.
CREATE TABLE entitlement_cache (
    scope_type         TEXT        NOT NULL,
    scope_id           BIGINT      NOT NULL,
    scope_secondary_id BIGINT      NOT NULL DEFAULT 0,
    tier               TEXT        NOT NULL,
    refreshed_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (scope_type, scope_id, scope_secondary_id)
);

-- M10.5: maps a SHA-256 hash of a Ko-fi email to a Discord snowflake so the
-- Ko-fi webhook can grant entitlements to the correct user without storing
-- plain-text email addresses.
CREATE TABLE kofi_links (
    id               SERIAL      PRIMARY KEY,
    email_hash       TEXT        NOT NULL UNIQUE,
    discord_user_id  BIGINT      NOT NULL,
    linked_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);
