-- Seed data for `tests/tree_fetch.rs`.
--
-- Guild 1 holds three things the component walk has to get right:
--
--   * Component A (users 10..15) -- the focus's family, reached through a mix
--     of partner and parent-child edges, including a step that is only
--     reachable *upwards* from the focus and one only reachable sideways
--     through a partner. A depth-first walk that refuses to expand partners'
--     parents would miss user 15.
--   * Component B (users 20, 21) -- a completely separate family in the same
--     guild, which must never appear in the focus's tree.
--   * A parent cycle (users 30 <-> 31) -- legal under the schema, which only
--     forbids self-loops. The recursive CTE must terminate on it.
--
-- Guild 2 re-uses user 10 to prove guild scoping: the same person's family in
-- another server must not leak into guild 1's tree.

INSERT INTO guilds (id) VALUES (1), (2);

INSERT INTO users (id, username)
VALUES
    (10, 'focus'),
    (11, 'partner_of_focus'),
    (12, 'child_of_focus'),
    (13, 'parent_of_focus'),
    (14, 'grandparent'),
    (15, 'partners_parent'),
    (20, 'stranger_a'),
    (21, 'stranger_b'),
    (30, 'cycle_a'),
    (31, 'cycle_b'),
    (40, 'other_guild_partner');

INSERT INTO family (guild_id, user_id)
VALUES
    (1, 10), (1, 11), (1, 12), (1, 13), (1, 14), (1, 15),
    (1, 20), (1, 21),
    (1, 30), (1, 31),
    (2, 10), (2, 40);

-- family_partners stores (LEAST, GREATEST); the CHECK enforces user_id < partner_id.
INSERT INTO family_partners (guild_id, user_id, partner_id)
VALUES
    (1, 10, 11),
    (1, 20, 21),
    (2, 10, 40);

INSERT INTO family_parent_child (guild_id, parent_id, child_id)
VALUES
    (1, 10, 12),
    (1, 13, 10),
    (1, 14, 13),
    (1, 15, 11),
    -- A two-node parent cycle. Representable, so it must not hang the walk.
    (1, 30, 31),
    (1, 31, 30);
