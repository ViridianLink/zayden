INSERT INTO
    web_sessions (token, discord_user_id, discord_access_token, expires_at)
VALUES
    ('live-token', 41, 'live-access-token', now() + interval '7 days'),
    ('expired-token', 42, 'expired-access-token', now() - interval '1 hour');
