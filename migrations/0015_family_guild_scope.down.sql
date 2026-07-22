DROP TABLE IF EXISTS family_settings;

DROP TABLE IF EXISTS family_blocks;

DROP TABLE IF EXISTS family_parent_child;

DROP TABLE IF EXISTS family_partners;

DROP TABLE IF EXISTS family;

CREATE TABLE family (
    user_id bigint PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE family_partners(
    user_id bigint NOT NULL REFERENCES family(user_id) ON DELETE CASCADE,
    partner_id bigint NOT NULL REFERENCES family(user_id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, partner_id),
    CHECK (user_id < partner_id)
);

CREATE TABLE family_parent_child(
    parent_id bigint NOT NULL REFERENCES family(user_id) ON DELETE CASCADE,
    child_id bigint NOT NULL REFERENCES family(user_id) ON DELETE CASCADE,
    PRIMARY KEY (parent_id, child_id),
    CHECK (parent_id <> child_id)
);

CREATE TABLE family_blocks(
    user_id bigint NOT NULL REFERENCES family(user_id) ON DELETE CASCADE,
    blocked_id bigint NOT NULL REFERENCES family(user_id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, blocked_id),
    CHECK (user_id <> blocked_id)
);

