-- Seed data for `tests/faq_article.rs`.
--
-- `faq_articles.guild_id` is `REFERENCES guilds (id)`, so both guilds must
-- exist before an article can be written. Two are seeded because the guild
-- scoping tests need a second, unrelated guild to read from.
--
-- Seeding lives here rather than in an inline `sqlx::query!` so the suite still
-- compiles under `SQLX_OFFLINE=true`: `cargo sqlx prepare` does not walk test
-- targets, so test-only SQL never reaches `.sqlx`.
INSERT INTO guilds (id)
VALUES
    (1),
    (2);
