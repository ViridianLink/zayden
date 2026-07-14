CREATE TABLE palworld_player_links(
    discord_id bigint PRIMARY KEY,
    player_uid text NOT NULL,
    in_game_name text NOT NULL,
    linked_at timestamptz NOT NULL DEFAULT now()
);

