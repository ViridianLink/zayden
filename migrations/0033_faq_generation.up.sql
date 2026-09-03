ALTER TABLE faq_settings
    ADD COLUMN auto_generate boolean NOT NULL DEFAULT FALSE;

ALTER TABLE support_settings
    DROP COLUMN faq_channel_id;

