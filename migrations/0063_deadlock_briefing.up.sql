ALTER TABLE deadlock_user_settings
    ADD COLUMN patch_briefing boolean NOT NULL DEFAULT TRUE;

CREATE TABLE deadlock_briefings_sent (
    patch_guid text NOT NULL REFERENCES deadlock_patches (guid) ON DELETE CASCADE,
    discord_id bigint NOT NULL,
    sent_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (patch_guid, discord_id)
);

CREATE INDEX deadlock_patches_unbriefed ON deadlock_patches (published_at)
WHERE
    briefed_at IS NULL AND diff IS NOT NULL;

