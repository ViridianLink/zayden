ALTER TABLE youtube_announce
    ADD COLUMN message text,
    ADD COLUMN ping_role_ids bigint[] NOT NULL DEFAULT '{}';

ALTER TABLE patreon_announce
    ADD COLUMN message text,
    ADD COLUMN ping_role_ids bigint[] NOT NULL DEFAULT '{}';

