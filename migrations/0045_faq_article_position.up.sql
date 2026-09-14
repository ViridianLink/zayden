ALTER TABLE faq_articles
    ADD COLUMN source_position smallint NOT NULL DEFAULT 0;

DROP INDEX faq_articles_thread_idx;

CREATE UNIQUE INDEX faq_articles_thread_idx ON faq_articles (guild_id, source_thread_id, source_position)
WHERE
    source_thread_id IS NOT NULL;

