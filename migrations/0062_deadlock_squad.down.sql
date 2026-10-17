DROP TABLE deadlock_squad_channel;

DROP TABLE deadlock_rank_snapshots;

ALTER TABLE deadlock_players
    DROP COLUMN announced_streak,
    DROP COLUMN badge;

ALTER TABLE deadlock_guild_members
    DROP COLUMN squad;

