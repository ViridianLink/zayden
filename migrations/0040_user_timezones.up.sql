ALTER TABLE lfg_user_settings RENAME TO user_timezones;

ALTER TABLE user_timezones RENAME COLUMN id TO user_id;

ALTER TABLE user_timezones
    ADD COLUMN updated_at timestamptz NOT NULL DEFAULT now();

