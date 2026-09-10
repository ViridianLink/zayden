-- Windows are baked in relative to now() because the sweeps compare against
-- now() in SQL; a test cannot move the clock, only the rows.
INSERT INTO users (id, username)
    VALUES (1, 'owner-one'), (2, 'owner-two'), (3, 'owner-three');

INSERT INTO hosted_servers (
    id, owner_id, game_key, plan, pelican_user_id, pelican_server_id,
    memory_mib, cpu_percent, disk_mib, price_cents, billing_source, claim_code,
    state, trial_ends_at, paid_until, reminded_at, suspended_at, delete_after)
VALUES
    -- Due to expire: a paid month that has run out.
    (1, 1, 'paper', 'small', 10, 101, 2048, 100, 10240, 250, 'kofi', 'AAAAA1',
     'active', NULL, now() - interval '1 day', now(), NULL, NULL),
    -- Due to expire: a priced trial that has run out.
    (2, 1, 'paper', 'medium', 10, 102, 4096, 150, 20480, 500, 'kofi', 'AAAAA2',
     'trial', now() - interval '1 hour', NULL, now(), NULL, NULL),
    -- Not due to expire, due to convert: an included plan's trial ending.
    (3, 2, 'paper', 'small', 20, 103, 2048, 100, 10240, 0, 'trial', 'AAAAA3',
     'trial', now() - interval '1 hour', NULL, now(), NULL, NULL),
    -- Not due: paid well into the future.
    (4, 2, 'palworld', 'large', 20, 104, 6144, 200, 30720, 750, 'kofi', 'AAAAA4',
     'active', NULL, now() + interval '20 days', now(), NULL, NULL),
    -- Due for deletion: suspended past its grace period.
    (5, 3, 'paper', 'small', 30, 105, 2048, 100, 10240, 250, 'kofi', 'AAAAA5',
     'suspended', NULL, now() - interval '10 days', now(), now() - interval '9 days',
     now() - interval '1 hour'),
    -- Not due for deletion: still inside the grace period.
    (6, 3, 'paper', 'small', 30, 106, 2048, 100, 10240, 250, 'kofi', 'AAAAA6',
     'suspended', NULL, now() - interval '2 days', now(), now() - interval '1 day',
     now() + interval '2 days'),
    -- Due for a reminder: lapses tomorrow and has not been told.
    (7, 1, 'paper', 'xl', 10, 107, 8192, 250, 40960, 1000, 'kofi', 'AAAAA7',
     'active', NULL, now() + interval '1 day', NULL, NULL, NULL),
    -- Not due for a reminder: already reminded about the same window.
    (8, 2, 'paper', 'xl', 20, 108, 8192, 250, 40960, 1000, 'kofi', 'AAAAA8',
     'active', NULL, now() + interval '1 day', now(), NULL, NULL),
    -- Subscription-billed, revalidated in Rust rather than by a predicate.
    (9, 3, 'paper', 'small', 30, 109, 2048, 100, 10240, 0, 'entitlement', 'AAAAA9',
     'active', NULL, NULL, now(), NULL, NULL),
    -- Already gone; no sweep may touch it again.
    (10, 3, 'paper', 'small', 30, 110, 2048, 100, 10240, 250, 'kofi', 'AAAA10',
     'deleted', NULL, now() - interval '30 days', now(), now() - interval '20 days', NULL),
    -- A live trial inside the reminder window. Never reminded: the window is
    -- measured in days and the trial in hours, so it would fire immediately.
    (11, 1, 'paper', 'small', 10, 111, 2048, 100, 10240, 250, 'trial', 'AAAA11',
     'trial', now() + interval '1 hour', NULL, NULL, NULL, NULL);

SELECT setval('hosted_servers_id_seq', (SELECT max(id) FROM hosted_servers));
