CREATE TABLE hosting_pelican_users (
    user_id bigint PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    pelican_user_id int NOT NULL UNIQUE,
    pelican_username text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE hosted_servers (
    id bigserial PRIMARY KEY,
    owner_id bigint NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    guild_id bigint REFERENCES guilds (id) ON DELETE SET NULL,
    game_key text NOT NULL,
    plan text NOT NULL,
    pelican_user_id int,
    pelican_server_id int UNIQUE,
    pelican_server_uuid text,
    memory_mib int NOT NULL,
    cpu_percent int NOT NULL,
    disk_mib int NOT NULL,
    price_cents int NOT NULL,
    billing_source text NOT NULL,
    claim_code text NOT NULL UNIQUE,
    address text,
    state text NOT NULL DEFAULT 'provisioning',
    failure_reason text,
    trial_ends_at timestamptz,
    paid_until timestamptz,
    reminded_at timestamptz,
    suspended_at timestamptz,
    delete_after timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT hosted_servers_state_check CHECK (state IN ('provisioning', 'trial', 'active', 'suspended', 'deleted', 'failed')),
    CONSTRAINT hosted_servers_billing_source_check CHECK (billing_source IN ('kofi', 'entitlement', 'trial')),
    CONSTRAINT hosted_servers_plan_check CHECK (plan IN ('small', 'medium', 'large', 'xl'))
);

CREATE INDEX hosted_servers_expiry_idx ON hosted_servers (coalesce(paid_until, trial_ends_at))
WHERE
    state IN ('trial', 'active');

CREATE INDEX hosted_servers_reminder_idx ON hosted_servers (paid_until)
WHERE
    state IN ('trial', 'active') AND reminded_at IS NULL AND paid_until IS NOT NULL;

CREATE INDEX hosted_servers_delete_idx ON hosted_servers (delete_after)
WHERE
    state = 'suspended' AND delete_after IS NOT NULL;

CREATE INDEX hosted_servers_owner_idx ON hosted_servers (owner_id)
WHERE
    state IN ('provisioning', 'trial', 'active', 'suspended');

CREATE TABLE hosting_payments (
    kofi_message_id text PRIMARY KEY,
    server_id bigint REFERENCES hosted_servers (id) ON DELETE SET NULL,
    kofi_email text,
    tier_name text,
    amount_cents int NOT NULL,
    currency text NOT NULL,
    matched_by text,
    received_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX hosting_payments_unmatched_idx ON hosting_payments (received_at)
WHERE
    server_id IS NULL;

