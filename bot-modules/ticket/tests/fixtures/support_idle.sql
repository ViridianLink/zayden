-- Written as a fixture rather than inline `query!` calls because
-- `cargo sqlx prepare` does not walk test targets, so test-only SQL would break
-- an offline build. Ages are baked in here for the same reason: the sweep has to
-- see rows older than the interval, and a test cannot issue its own UPDATE.
INSERT INTO guilds (id)
    VALUES (1), (2);

INSERT INTO support_settings (guild_id, support_channel_id, idle_enabled, idle_after_secs)
    VALUES (1, 500, TRUE, 3600),
    (2, 501, FALSE, 3600);

INSERT INTO guild_support_roles (guild_id, role_id)
    VALUES (1, 100);

INSERT INTO support_thread_activity (thread_id, guild_id, op_id, helper_id, waiting_on_helper, since, nudged_at, paused)
    VALUES
        -- Due: nobody has answered this one at all.
        (10, 1, 1000, NULL, TRUE, now() - interval '10 days', NULL, FALSE),
        -- Due: a helper spoke last, so the poster owes a reply.
        (11, 1, 1000, 2000, FALSE, now() - interval '10 days', NULL, FALSE),
        -- Due: the poster spoke last and a known helper owes a reply.
        (12, 1, 1000, 2000, TRUE, now() - interval '10 days', NULL, FALSE),
        -- Not due: somebody just posted.
        (13, 1, 1000, NULL, TRUE, now(), NULL, FALSE),
        -- Not due: already nudged for this turn.
        (14, 1, 1000, NULL, TRUE, now() - interval '10 days', now(), FALSE),
        -- Not due: solved or closed.
        (15, 1, 1000, 2000, TRUE, now() - interval '10 days', NULL, TRUE),
        -- Not due: the guild has idle reminders switched off.
        (20, 2, 1000, NULL, TRUE, now() - interval '10 days', NULL, FALSE);
