INSERT INTO users (id, username)
    VALUES (1, 'payer'), (2, 'stranger');

-- The hash is opaque to this crate: the dashboard computes it, hosting only
-- looks it up.
INSERT INTO kofi_links (email_hash, discord_user_id)
    VALUES ('hash-of-payer', 1);

INSERT INTO hosted_servers (
    id, owner_id, game_key, plan, pelican_user_id, pelican_server_id,
    memory_mib, cpu_percent, disk_mib, price_cents, billing_source, claim_code,
    state, trial_ends_at, paid_until)
VALUES
    -- The payer's £2.50 server, lapsing first.
    (1, 1, 'paper', 'small', 10, 101, 2048, 100, 10240, 250, 'kofi', 'BCDFGH',
     'suspended', NULL, now() - interval '1 day'),
    -- A second £2.50 server, so a bare tier match is ambiguous and the claim
    -- code has something to disambiguate against.
    (2, 1, 'palworld', 'small', 10, 102, 2048, 100, 10240, 250, 'kofi', 'JKMNPQ',
     'active', NULL, now() + interval '10 days'),
    -- A £7.50 server, which a £2.50 payment must never settle.
    (3, 1, 'paper', 'large', 10, 103, 6144, 200, 30720, 750, 'kofi', 'RSTVWX',
     'active', NULL, now() + interval '5 days'),
    -- Someone else's server, unreachable from this payer's email.
    (4, 2, 'paper', 'small', 20, 104, 2048, 100, 10240, 250, 'kofi', 'YZ2345',
     'active', NULL, now() + interval '5 days');

SELECT setval('hosted_servers_id_seq', (SELECT max(id) FROM hosted_servers));
