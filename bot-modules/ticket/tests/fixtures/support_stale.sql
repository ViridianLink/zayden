-- Ages are baked in here for the same reason as `support_idle_close.sql`: the
-- stale sweep has to see silence older than the guild's window, and a test
-- cannot issue its own UPDATE against a fixture row.
INSERT INTO guilds (id)
    VALUES (1), (2), (3);

INSERT INTO support_settings (guild_id, support_channel_id, stale_enabled, stale_tag_id, stale_after_secs)
    VALUES
        -- Stale tagging on, with a tag to apply.
        (1, 500, TRUE, 800, 3600),
        -- On, but no tag configured, so there is nothing to apply.
        (2, 501, TRUE, NULL, 3600),
        -- Off.
        (3, 502, FALSE, 800, 3600);

INSERT INTO guild_support_roles (guild_id, role_id)
    VALUES (1, 100);

INSERT INTO support_thread_activity (thread_id, guild_id, op_id, helper_id, waiting_on_helper, since, staled_at, paused)
    VALUES
        -- Due: a helper spoke last and the poster has been quiet for days.
        (10, 1, 1000, 2000, FALSE, now() - interval '10 days', NULL, FALSE),
        -- Due: the second one proves the claim is not a single row.
        (11, 1, 1001, 2000, FALSE, now() - interval '10 days', NULL, FALSE),
        -- Not due: the support team owes the reply.
        (12, 1, 1000, 2000, TRUE, now() - interval '30 days', NULL, FALSE),
        -- Not due: nobody has answered yet, which is also the team's turn.
        (13, 1, 1000, NULL, TRUE, now() - interval '30 days', NULL, FALSE),
        -- Not due: the poster went quiet minutes ago.
        (14, 1, 1000, 2000, FALSE, now(), NULL, FALSE),
        -- Not due: already solved or closed.
        (15, 1, 1000, 2000, FALSE, now() - interval '10 days', NULL, TRUE),
        -- Not due: already tagged stale.
        (16, 1, 1000, 2000, FALSE, now() - interval '10 days', now() - interval '1 day', FALSE),
        -- Cleared: tagged stale, then somebody posted.
        (17, 1, 1000, 2000, FALSE, now() - interval '1 hour', now() - interval '1 day', FALSE),
        -- Not due: the guild configured no tag to apply.
        (20, 2, 1000, 2000, FALSE, now() - interval '10 days', NULL, FALSE),
        -- Not due: the guild has stale tagging switched off.
        (30, 3, 1000, 2000, FALSE, now() - interval '10 days', NULL, FALSE);
