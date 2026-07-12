ALTER TABLE guild_channels RENAME TO channels_settings;

ALTER TRIGGER guild_channels_notify ON channels_settings RENAME TO channels_settings_notify;

ALTER TABLE lfg_user_config RENAME TO lfg_user_settings;

CREATE TYPE infraction_kind AS ENUM(
    'Warn',
    'Mute',
    'Kick',
    'SoftBan',
    'Ban'
);

ALTER TABLE infractions
    ALTER COLUMN infraction_type TYPE infraction_kind
    USING infraction_type::infraction_kind;

