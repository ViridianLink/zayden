ALTER TABLE user_timezones
    DROP COLUMN updated_at;

ALTER TABLE user_timezones
    RENAME COLUMN user_id TO id;

ALTER TABLE user_timezones RENAME TO lfg_user_settings;
