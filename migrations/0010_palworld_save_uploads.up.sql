CREATE TABLE palworld_save_uploads(
    discord_id bigint PRIMARY KEY,
    file_path text NOT NULL,
    uploaded_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL
);

