INSERT INTO account_links (discord_id, provider, external_id)
    VALUES
        (10, 'steam', '76561197960266729'),
        (11, 'steam', '76561197960266730'),
        (12, 'steam', '76561197960266731'),
        (13, 'steam', '76561197960266732'),
        (14, 'steam', '76561197960266733');

INSERT INTO deadlock_players (account_id, discord_id)
    VALUES (1001, 10), (1002, 11), (1003, 12), (1004, 13), (1005, 14);

INSERT INTO deadlock_patches (guid, title, link, published_at, diff, briefed_at)
    VALUES
        ('g1', 'Update 10 October', 'https://example.test/g1', '2026-10-10T12:00:00Z',
         '{"heroes":[{"hero_id":1,"name":"Infernus","presence":"changed","stats":[],"weapon":[],"abilities":[]}],"items":[]}',
         NULL),
        ('g0', 'Update 10 October (no diff)', 'https://example.test/g0', '2026-10-10T13:00:00Z',
         NULL,
         NULL),
        ('g2', 'Update 11 October', 'https://example.test/g2', '2026-10-11T12:00:00Z',
         '{"heroes":[],"items":[]}',
         now());

-- 10 played hero 1 twice inside the 14 days before g1.
-- 11 opted out. 12 played again after g1. 13 only played before the window.
-- 14 was already briefed about g1.
INSERT INTO deadlock_matches_seen (account_id, match_id, hero_id, won, start_time)
    VALUES
        (1001, 1, 1, TRUE, '2026-10-08T08:00:00Z'),
        (1001, 2, 1, FALSE, '2026-10-09T08:00:00Z'),
        (1002, 3, 1, TRUE, '2026-10-08T08:00:00Z'),
        (1003, 4, 1, TRUE, '2026-10-08T08:00:00Z'),
        (1003, 5, 1, TRUE, '2026-10-11T08:00:00Z'),
        (1004, 6, 1, TRUE, '2026-09-20T08:00:00Z'),
        (1005, 7, 1, TRUE, '2026-10-08T08:00:00Z');

INSERT INTO deadlock_user_settings (discord_id, patch_briefing)
    VALUES (11, FALSE);

INSERT INTO deadlock_briefings_sent (patch_guid, discord_id)
    VALUES ('g1', 14);
