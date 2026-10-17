-- The test runs at 2026-10-17T00:00:00Z.
INSERT INTO deadlock_api_cache (key, kind, body, expires_at)
    VALUES
        ('expired-ten-days', 'asset', '{}', '2026-10-07T00:00:00Z'),
        ('expired-two-days', 'asset', '{}', '2026-10-15T00:00:00Z');

INSERT INTO account_links (discord_id, provider, external_id)
    VALUES (10, 'steam', '76561197960266729');

INSERT INTO deadlock_players (account_id, discord_id)
    VALUES (1001, 10);

INSERT INTO deadlock_rank_snapshots (account_id, captured_at, badge)
    VALUES
        (1001, '2026-01-01T00:00:00Z', 11),
        (1001, '2026-10-01T00:00:00Z', 12);

INSERT INTO deadlock_matches_seen (account_id, match_id, hero_id, won, start_time)
    VALUES
        (1001, 1, 1, TRUE, '2026-06-01T00:00:00Z'),
        (1001, 2, 1, TRUE, '2026-10-01T00:00:00Z');

INSERT INTO guilds (id)
    VALUES (1);

INSERT INTO deadlock_guild_posts (guild_id, match_id, posted_at)
    VALUES
        (1, 1, '2026-06-01T00:00:00Z'),
        (1, 2, '2026-10-01T00:00:00Z');

INSERT INTO deadlock_patches (guid, title, link, published_at)
    VALUES ('g1', 'Update', 'https://example.test/g1', '2026-06-01T00:00:00Z');

INSERT INTO deadlock_briefings_sent (patch_guid, discord_id, sent_at)
    VALUES
        ('g1', 10, '2026-06-01T00:00:00Z'),
        ('g1', 11, '2026-10-01T00:00:00Z');

INSERT INTO deadlock_asset_snapshots (client_version, heroes, items)
    SELECT version, '[]', '[]' FROM generate_series(1, 8) AS version;

INSERT INTO deadlock_watch_state (key, client_version)
    VALUES ('baseline', 2), ('latest', 8);
