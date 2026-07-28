CREATE TABLE web_user_roles(
    discord_user_id bigint NOT NULL,
    role text NOT NULL,
    granted_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (discord_user_id, ROLE)
);

INSERT INTO web_user_roles(discord_user_id, role)
    VALUES (211486447369322506, 'admin')
ON CONFLICT
    DO NOTHING;

