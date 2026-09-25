-- Seed data for `tests/store.rs`.
--
-- Guilds 1 and 2 share channel UCshared (guild 2 has no announce channel set);
-- guild 3 follows UCother. UCorphan has no connection at all.
INSERT INTO guilds (id)
VALUES
    (1),
    (2),
    (3);

INSERT INTO youtube_channels (channel_id, title, uploads_playlist_id, websub_secret, websub_expires_at, seeded_at)
VALUES
    ('UCshared', 'Shared Creator', 'UUshared', 'secret-shared', now() + interval '4 days', now() - interval '10 days'),
    ('UCother', 'Other Creator', 'UUother', 'secret-other', now() + interval '1 hour', NULL),
    ('UCorphan', 'Orphan', 'UUorphan', 'secret-orphan', NULL, NULL);

INSERT INTO youtube_connections (guild_id, channel_id, connected_by)
VALUES
    (1, 'UCshared', 9001),
    (2, 'UCshared', 9002),
    (3, 'UCother', 9003);

INSERT INTO youtube_videos (video_id, channel_id, title, published_at, announced_at)
VALUES
    ('v-old', 'UCshared', 'Already announced', now() - interval '3 days', now() - interval '3 days'),
    ('v-first', 'UCshared', 'First pending', now() - interval '2 hours', NULL),
    ('v-second', 'UCshared', 'Second pending', now() - interval '1 hour', NULL);

INSERT INTO youtube_announce (guild_id, channel_id)
VALUES
    (1, 1001),
    (3, 1003);
