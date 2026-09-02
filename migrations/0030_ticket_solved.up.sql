ALTER TABLE support_settings
    ADD COLUMN solved_tag_id bigint,
    ADD COLUMN helper_role_id bigint,
    ADD COLUMN solved_archive_secs int NOT NULL DEFAULT 60;

CREATE TABLE guild_helper_links (
    guild_id bigint NOT NULL REFERENCES guilds (id) ON DELETE CASCADE,
    user_id bigint NOT NULL,
    link text NOT NULL,
    PRIMARY KEY (guild_id, user_id)
);

