ALTER TABLE infractions
    ALTER COLUMN infraction_type TYPE VARCHAR(255)
    USING infraction_type::text;

DROP TYPE infraction_kind;

ALTER TABLE lfg_user_settings RENAME TO lfg_user_config;

ALTER TRIGGER channels_settings_notify ON channels_settings RENAME TO guild_channels_notify;

ALTER TABLE channels_settings RENAME TO guild_channels;

