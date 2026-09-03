ALTER TABLE support_settings
    ADD COLUMN faq_channel_id bigint;

ALTER TABLE faq_settings
    DROP COLUMN auto_generate;

