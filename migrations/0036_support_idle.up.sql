INSERT INTO guild_support_roles (guild_id, role_id)
SELECT
    guild_id,
    helper_role_id
FROM
    support_settings
WHERE
    helper_role_id IS NOT NULL
ON CONFLICT (guild_id,
    role_id)
    DO NOTHING;

ALTER TABLE support_settings
    DROP COLUMN helper_role_id,
    ADD COLUMN idle_enabled boolean NOT NULL DEFAULT FALSE,
    ADD COLUMN idle_after_secs int NOT NULL DEFAULT 172800 CHECK (idle_after_secs >= 3600);

CREATE TABLE support_thread_activity (
    thread_id bigint PRIMARY KEY,
    guild_id bigint NOT NULL REFERENCES guilds (id) ON DELETE CASCADE,
    op_id bigint NOT NULL,
    helper_id bigint,
    waiting_on_helper boolean NOT NULL DEFAULT TRUE,
    since timestamptz NOT NULL DEFAULT now(),
    nudged_at timestamptz,
    paused boolean NOT NULL DEFAULT FALSE,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX support_thread_activity_due_idx ON support_thread_activity (guild_id, since)
WHERE
    nudged_at IS NULL AND NOT paused;

