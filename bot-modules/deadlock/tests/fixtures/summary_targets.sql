INSERT INTO guilds (id)
    VALUES (1), (2), (3), (4), (5), (6), (7);

INSERT INTO deadlock_summary_channel (guild_id, channel_id)
    VALUES (1, 101), (2, 102), (3, 103), (4, 104), (5, 105), (6, 106);

-- Member 10 shares in every guild except 4; member 11 only in guild 1.
INSERT INTO deadlock_guild_members (guild_id, discord_id, share_summaries)
    VALUES
        (1, 10, TRUE),
        (2, 10, TRUE),
        (3, 10, TRUE),
        (4, 10, FALSE),
        (5, 10, TRUE),
        (6, 10, TRUE),
        (7, 10, TRUE),
        (1, 11, TRUE);

-- Zayden is application 1000; guild 6 is served by the custom bot 2000.
INSERT INTO guild_presence (guild_id, application_id, left_at)
    VALUES
        (1, 1000, NULL),
        (2, 1000, NULL),
        (3, 1000, NULL),
        (4, 1000, NULL),
        -- Zayden left guild 5.
        (5, 1000, now()),
        (6, 1000, NULL),
        (6, 2000, NULL),
        (7, 1000, NULL);

INSERT INTO guild_modules (guild_id, module, enabled)
    VALUES
        -- Switched on explicitly.
        (2, 'deadlock', TRUE),
        -- Switched off.
        (3, 'deadlock', FALSE);

INSERT INTO custom_bots (application_id, bot_user_id, name, public_key, token_ct, token_nonce, key_id, registered_by)
    VALUES (2000, 2001, 'Custom', 'key', '\x00'::bytea, '\x00'::bytea, 1, 1);

INSERT INTO custom_bot_guilds (guild_id, application_id, attached_by)
    VALUES (6, 2000, 1);

INSERT INTO entitlements (provider, external_id, scope_type, scope_id, tier)
    VALUES ('test', 'guild-6', 'guild', 6, 'ultra');
