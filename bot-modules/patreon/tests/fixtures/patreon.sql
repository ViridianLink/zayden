-- Seed data for `tests/announce.rs`.
--
-- Guild 1 and guild 2 are both connected to campaign 100 (one creator, two
-- servers) and share its posts; guild 2 only wants public ones. Guild 3 is
-- connected to campaign 300 but has no channel set, and guild 4's connection is
-- disabled, so neither should be polled or announced to.
INSERT INTO guilds (id)
VALUES
    (1),
    (2),
    (3),
    (4);

INSERT INTO patreon_campaigns (campaign_id, next_cursor, seeded_at)
VALUES
    ('100', 'cursor-1', now() - interval '1 day'),
    ('300', NULL, NULL),
    ('400', NULL, NULL);

INSERT INTO patreon_oauth (guild_id, campaign_id, creator_name, access_token, refresh_token, expires_at, webhook_secret, connected_by, disabled_at)
VALUES
    (1, '100', 'Example Creator', 'access-1', 'refresh-1', now() + interval '1 hour', 'secret-1', 9001, NULL),
    (2, '100', 'Example Creator', 'access-2', 'refresh-2', now() + interval '1 hour', 'secret-2', 9002, NULL),
    (3, '300', 'No Channel', 'access-3', 'refresh-3', now() + interval '1 hour', NULL, 9003, NULL),
    (4, '400', 'Revoked', 'access-4', 'refresh-4', now() - interval '1 hour', 'secret-4', 9004, now());

INSERT INTO patreon_posts (post_id, campaign_id, title, url, content_html, is_public, published_at, announced_at)
VALUES
    ('p-old', '100', 'Already announced', 'https://patreon.test/p-old', '<p>old</p>', TRUE, now() - interval '3 days', now() - interval '3 days'),
    ('p-public', '100', 'Public update', 'https://patreon.test/p-public', '<p>public</p>', TRUE, now() - interval '2 days', NULL),
    ('p-patrons', '100', 'Patrons only', 'https://patreon.test/p-patrons', '<p>secret</p>', FALSE, now() - interval '1 day', NULL);

-- Guild 3 is deliberately absent: a connection without a channel is not a
-- subscriber, and must not drag its campaign into the poll.
INSERT INTO patreon_announce (guild_id, channel_id, public_only)
VALUES
    (1, 1001, FALSE),
    (2, 1002, TRUE),
    (4, 1004, FALSE);
