-- Seed data for `tests/give_star.rs`.
--
-- `gold_stars.id` is `REFERENCES users (id)`, so every actor needs a `users`
-- row before it can hold stars. `GoldStarRow::give_star` does **not** create
-- one (unlike `levels`/`family`, which `INSERT INTO users … ON CONFLICT DO
-- NOTHING` first), so the fixture supplies them — see the FK note in
-- `give_star.rs`.
INSERT INTO users(id, username)
VALUES
    (100, 'paid-author'),
(200, 'empty-author'),
(300, 'expired-window-author'),
(400, 'second-paid-author'),
(900, 'target');

-- `last_free_star = now()` means the 24h free star is spent; the row must fall
-- through to the paid path. `now() - 25 hours` means the window has reopened.
INSERT INTO gold_stars(id, number_of_stars, given_stars, received_stars, last_free_star)
VALUES
    (100, 2, 0, 0, now()),
(200, 0, 0, 0, now()),
(300, 0, 0, 0, now() - INTERVAL '25 hours'),
(400, 1, 0, 0, now());

-- User 900 deliberately has **no** `gold_stars` row: it exercises the credit's
-- INSERT arm, and lets a failed give assert that no row was minted at all.
