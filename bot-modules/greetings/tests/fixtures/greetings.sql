-- Seed data for `tests/images.rs`.
--
-- `greeting_images.guild_id` is `REFERENCES guilds (id)`, so a guild row must
-- exist before it can hold images. `GreetingImage::add` seeds one itself, so
-- guild 1 is pre-seeded to start the list tests from a known state while guild
-- 3 is deliberately omitted to exercise that seeding path.
INSERT INTO guilds (id)
VALUES
    (1),
    (2);

INSERT INTO greeting_images (guild_id, kind, url)
VALUES
    (1, 'morning', 'https://example.com/sunrise-1.gif'),
    (1, 'morning', 'https://example.com/sunrise-2.gif'),
    (1, 'night', 'https://example.com/moon-1.gif'),
    (2, 'morning', 'https://example.com/other-guild.gif');

