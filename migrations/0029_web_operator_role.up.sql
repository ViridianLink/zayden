INSERT INTO web_user_roles (discord_user_id, role)
    VALUES (211486447369322506, 'operator')
ON CONFLICT
    DO NOTHING;

