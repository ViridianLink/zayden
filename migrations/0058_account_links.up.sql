CREATE TABLE account_links (
    discord_id bigint NOT NULL,
    provider text NOT NULL CHECK (provider IN ('steam')),
    external_id text NOT NULL,
    linked_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (discord_id, provider),
    UNIQUE (provider, external_id)
);

