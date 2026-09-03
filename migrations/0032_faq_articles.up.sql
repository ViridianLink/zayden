CREATE TABLE faq_articles (
    id int GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    guild_id bigint NOT NULL REFERENCES guilds (id) ON DELETE CASCADE,
    title text NOT NULL,
    summary text NOT NULL,
    content text NOT NULL,
    category text,
    tags text[] NOT NULL DEFAULT '{}',
    source_thread_id bigint,
    generated boolean NOT NULL DEFAULT TRUE,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    search tsvector GENERATED ALWAYS AS (setweight(to_tsvector('english'::regconfig, title), 'A') || setweight(to_tsvector('english'::regconfig, summary), 'B') || setweight(to_tsvector('english'::regconfig, content), 'C')) STORED
);

CREATE INDEX faq_articles_search_idx ON faq_articles USING GIN (search);

CREATE INDEX faq_articles_guild_idx ON faq_articles (guild_id, updated_at DESC);

CREATE UNIQUE INDEX faq_articles_thread_idx ON faq_articles (guild_id, source_thread_id)
WHERE
    source_thread_id IS NOT NULL;

