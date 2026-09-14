UPDATE
    faq_articles
SET
    source_thread_id = NULL
WHERE
    source_position <> 0;

DROP INDEX faq_articles_thread_idx;

CREATE UNIQUE INDEX faq_articles_thread_idx ON faq_articles (guild_id, source_thread_id)
WHERE
    source_thread_id IS NOT NULL;

ALTER TABLE faq_articles
    DROP COLUMN source_position;

