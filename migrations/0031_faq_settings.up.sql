CREATE TABLE faq_settings (
    guild_id bigint PRIMARY KEY REFERENCES guilds (id) ON DELETE CASCADE,
    enabled boolean NOT NULL DEFAULT FALSE,
    auto_triage boolean NOT NULL DEFAULT FALSE,
    wiki_url text,
    wiki_api_key text,
    wiki_locale text NOT NULL DEFAULT 'en',
    max_results integer NOT NULL DEFAULT 5,
    answer_max_tokens integer NOT NULL DEFAULT 500,
    answer_temperature real NOT NULL DEFAULT 0.2,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE OR REPLACE TRIGGER faq_settings_notify
    AFTER INSERT OR UPDATE OR DELETE ON faq_settings
    FOR EACH ROW
    EXECUTE FUNCTION notify_config_changed ();

