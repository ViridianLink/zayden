CREATE TABLE marathon_news_seen(
    source text PRIMARY KEY,
    last_id text,
    updated_at timestamptz NOT NULL DEFAULT now()
);

