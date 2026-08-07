-- Seed data for `tests/higher_lower_winners.sql`.
--
-- `gambling_stats.user_id` is `REFERENCES gambling (user_id)`, which in turn
-- references `users (id)`, so every player needs a row in all three tables.
--
-- Users 100/200 played this week (non-zero weekly score); 300/400/500 did not
-- and sit at the post-reset default of 0. Their all-time
-- `higher_or_lower_score` is deliberately high so a query that forgets to
-- filter on the *weekly* column can't accidentally look correct.
INSERT INTO users(id, username)
VALUES
    (100, 'weekly-first'),
(200, 'weekly-second'),
(300, 'idle-veteran'),
(400, 'idle-rookie'),
(500, 'idle-lurker');

INSERT INTO gambling(user_id)
VALUES
    (100),
(200),
(300),
(400),
(500);

INSERT INTO gambling_stats(user_id, higher_or_lower_score, weekly_higher_or_lower_score)
VALUES
    (100, 9, 9),
(200, 4, 4),
(300, 147, 0),
(400, 32, 0),
(500, 27, 0);

