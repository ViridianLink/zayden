-- A separate fixture from `support_idle`, which `idle_activity` asserts an
-- exact claim set against. Ages are baked in here because the close sweep has
-- to see reminders older than the grace period and a test cannot issue its own
-- UPDATE - see the note at the top of `support_idle.sql`.
INSERT INTO guilds (id)
    VALUES (1), (2), (3);

INSERT INTO support_settings (guild_id, support_channel_id, closed_tag_id, idle_enabled, idle_after_secs, idle_close_enabled, idle_close_after_secs)
    VALUES
        -- Reminders and auto-close both on.
        (1, 500, 700, TRUE, 3600, TRUE, 3600),
        -- Reminders on, auto-close off.
        (2, 501, NULL, TRUE, 3600, FALSE, 3600),
        -- Both off, but auto-close switched on anyway.
        (3, 502, NULL, FALSE, 3600, TRUE, 3600);

INSERT INTO guild_support_roles (guild_id, role_id)
    VALUES (1, 100);

INSERT INTO support_thread_activity (thread_id, guild_id, op_id, helper_id, waiting_on_helper, since, nudged_at, paused)
    VALUES
        -- Due: a helper spoke last, the poster was reminded and stayed quiet.
        (10, 1, 1000, 2000, FALSE, now() - interval '10 days', now() - interval '2 days', FALSE),
        -- Due: same, and the second one proves the batch is not a single row.
        (11, 1, 1001, 2000, FALSE, now() - interval '10 days', now() - interval '5 days', FALSE),
        -- Not due: the support team owes the reply, so this is never closed.
        (12, 1, 1000, 2000, TRUE, now() - interval '30 days', now() - interval '20 days', FALSE),
        -- Not due: nobody has answered at all, which is also the team's turn.
        (13, 1, 1000, NULL, TRUE, now() - interval '30 days', now() - interval '20 days', FALSE),
        -- Not due: the reminder went out minutes ago.
        (14, 1, 1000, 2000, FALSE, now() - interval '10 days', now(), FALSE),
        -- Not due: never reminded, so the grace period has not started.
        (15, 1, 1000, 2000, FALSE, now() - interval '10 days', NULL, FALSE),
        -- Not due: already solved or closed.
        (16, 1, 1000, 2000, FALSE, now() - interval '10 days', now() - interval '2 days', TRUE),
        -- Not due: the guild has auto-close switched off.
        (20, 2, 1000, 2000, FALSE, now() - interval '10 days', now() - interval '2 days', FALSE),
        -- Not due: the guild has reminders switched off.
        (30, 3, 1000, 2000, FALSE, now() - interval '10 days', now() - interval '2 days', FALSE);
