//! Who serves a guild: its custom bot only while it is active, entitled to
//! guild-scope Ultra and present; Zayden otherwise.

use jiff::Timestamp;
use secrecy::SecretString;
use sqlx::PgPool;
use zayden_app::custom_bots::{
    CustomBotStatus,
    CustomBotStore,
    Keyring,
    NewCustomBot,
};
use zayden_app::guilds::GuildPresence;
use zayden_app::serving::{ServingBot, ServingKind, ServingResolver};

const KEY: &str = "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=";
const ZAYDEN: u64 = 100;
const LLAMA: i64 = 200;
const GUILD: i64 = 10;
const SECOND_GUILD: i64 = 20;
const ADMIN: i64 = 1;

const fn joined() -> Timestamp {
    Timestamp::constant(1_750_000_000, 0)
}

#[expect(
    clippy::unwrap_used,
    reason = "a free helper sits outside the #[test] items clippy.toml exempts"
)]
async fn serving(pool: &PgPool, guild_id: i64) -> ServingBot {
    ServingResolver::new(pool.clone(), ZAYDEN).resolve(guild_id).await.unwrap()
}

const fn zayden(present: bool) -> ServingBot {
    ServingBot { application_id: ZAYDEN, kind: ServingKind::Zayden, present }
}

const fn llama() -> ServingBot {
    ServingBot {
        application_id: LLAMA.cast_unsigned(),
        kind: ServingKind::Custom,
        present: true,
    }
}

#[expect(
    clippy::unwrap_used,
    reason = "a free helper sits outside the #[test] items clippy.toml exempts"
)]
async fn register(pool: &PgPool, guilds: &[i64]) {
    let keys = Keyring::parse(&format!("1:{KEY}"), "1").unwrap();
    let sealed = keys.seal(LLAMA, &SecretString::from("t".to_owned())).unwrap();
    let bot = NewCustomBot {
        application_id: LLAMA,
        bot_user_id: LLAMA,
        name: "LlamaBot",
        avatar: None,
        public_key: "public",
        registered_by: ADMIN,
    };
    CustomBotStore::upsert(pool, &bot, &sealed).await.unwrap();
    for &guild in guilds {
        CustomBotStore::attach(pool, guild, LLAMA, ADMIN).await.unwrap();
    }
}

#[expect(
    clippy::unwrap_used,
    reason = "a free helper sits outside the #[test] items clippy.toml exempts"
)]
async fn grant(
    pool: &PgPool,
    scope_type: &str,
    scope_id: i64,
    tier: &str,
    expired_days_ago: Option<i32>,
) {
    sqlx::query!(
        "INSERT INTO entitlements
             (provider, external_id, scope_type, scope_id, tier, expires_at)
         VALUES ('test', $1, $2, $3, $4,
                 now() - make_interval(days => $5))",
        format!("{scope_type}:{scope_id}:{tier}"),
        scope_type,
        scope_id,
        tier,
        expired_days_ago,
    )
    .execute(pool)
    .await
    .unwrap();
}

#[expect(
    clippy::unwrap_used,
    reason = "a free helper sits outside the #[test] items clippy.toml exempts"
)]
async fn join(pool: &PgPool, guild_id: i64, application_id: i64) {
    GuildPresence::joined(pool, guild_id, application_id, joined()).await.unwrap();
}

/// A custom bot that is registered, entitled and present in `GUILD`.
async fn serving_llama(pool: &PgPool) {
    register(pool, &[GUILD]).await;
    grant(pool, "guild", GUILD, "ultra", None).await;
    join(pool, GUILD, LLAMA).await;
}

#[sqlx::test(migrations = "../migrations")]
async fn a_guild_without_a_custom_bot_is_served_by_zayden(pool: PgPool) {
    join(&pool, GUILD, ZAYDEN.cast_signed()).await;

    assert_eq!(serving(&pool, GUILD).await, zayden(true));
}

#[sqlx::test(migrations = "../migrations")]
async fn zayden_is_reported_absent_when_it_is_not_in_the_guild(pool: PgPool) {
    assert_eq!(serving(&pool, GUILD).await, zayden(false));
}

#[sqlx::test(migrations = "../migrations")]
async fn a_present_entitled_custom_bot_serves(pool: PgPool) {
    serving_llama(&pool).await;

    assert_eq!(serving(&pool, GUILD).await, llama());
}

#[sqlx::test(migrations = "../migrations")]
async fn a_custom_bot_not_yet_invited_leaves_zayden_serving(pool: PgPool) {
    register(&pool, &[GUILD]).await;
    grant(&pool, "guild", GUILD, "ultra", None).await;
    join(&pool, GUILD, ZAYDEN.cast_signed()).await;

    assert_eq!(serving(&pool, GUILD).await, zayden(true));
}

#[sqlx::test(migrations = "../migrations")]
async fn the_custom_bot_wins_when_both_are_present(pool: PgPool) {
    join(&pool, GUILD, ZAYDEN.cast_signed()).await;
    serving_llama(&pool).await;

    assert_eq!(serving(&pool, GUILD).await, llama());
}

#[sqlx::test(migrations = "../migrations")]
async fn a_kicked_custom_bot_hands_back_to_zayden(pool: PgPool) {
    serving_llama(&pool).await;
    GuildPresence::left(&pool, GUILD, LLAMA).await.unwrap();

    assert_eq!(serving(&pool, GUILD).await, zayden(false));
}

#[sqlx::test(migrations = "../migrations")]
async fn a_revoked_custom_bot_hands_back_to_zayden(pool: PgPool) {
    serving_llama(&pool).await;
    CustomBotStore::set_status(&pool, LLAMA, CustomBotStatus::Revoked)
        .await
        .unwrap();

    assert_eq!(serving(&pool, GUILD).await, zayden(false));
}

#[sqlx::test(migrations = "../migrations")]
async fn a_lapsed_entitlement_keeps_serving_during_the_grace_window(pool: PgPool) {
    register(&pool, &[GUILD]).await;
    grant(&pool, "guild", GUILD, "ultra", Some(2)).await;
    join(&pool, GUILD, LLAMA).await;

    assert_eq!(serving(&pool, GUILD).await, llama());
}

#[sqlx::test(migrations = "../migrations")]
async fn a_lapsed_entitlement_past_the_grace_window_hands_back_to_zayden(
    pool: PgPool,
) {
    register(&pool, &[GUILD]).await;
    grant(&pool, "guild", GUILD, "ultra", Some(4)).await;
    join(&pool, GUILD, LLAMA).await;

    assert_eq!(serving(&pool, GUILD).await, zayden(false));
}

#[sqlx::test(migrations = "../migrations")]
async fn a_user_scope_ultra_does_not_qualify(pool: PgPool) {
    register(&pool, &[GUILD]).await;
    grant(&pool, "user", ADMIN, "ultra", None).await;
    join(&pool, GUILD, LLAMA).await;

    assert_eq!(serving(&pool, GUILD).await, zayden(false));
}

#[sqlx::test(migrations = "../migrations")]
async fn a_guild_pro_entitlement_does_not_qualify(pool: PgPool) {
    register(&pool, &[GUILD]).await;
    grant(&pool, "guild", GUILD, "pro", None).await;
    join(&pool, GUILD, LLAMA).await;

    assert_eq!(serving(&pool, GUILD).await, zayden(false));
}

#[sqlx::test(migrations = "../migrations")]
async fn each_guild_of_a_shared_bot_is_gated_by_its_own_entitlement(pool: PgPool) {
    register(&pool, &[GUILD, SECOND_GUILD]).await;
    grant(&pool, "guild", GUILD, "ultra", None).await;
    join(&pool, GUILD, LLAMA).await;
    join(&pool, SECOND_GUILD, LLAMA).await;

    assert_eq!(serving(&pool, GUILD).await, llama());
    assert_eq!(serving(&pool, SECOND_GUILD).await, zayden(false));
}

#[sqlx::test(migrations = "../migrations")]
async fn a_detached_guild_hands_back_to_zayden(pool: PgPool) {
    serving_llama(&pool).await;
    CustomBotStore::detach(&pool, GUILD).await.unwrap();

    assert_eq!(serving(&pool, GUILD).await, zayden(false));
}

#[sqlx::test(migrations = "../migrations")]
async fn the_llama_guilds_are_entitled_by_migration(pool: PgPool) {
    for guild in [1_133_034_263_579_734_037_i64, 935_189_797_528_555_610] {
        let entitled = sqlx::query_scalar!(
            r#"SELECT custom_bot_entitled($1) AS "entitled!""#,
            guild
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert!(entitled, "guild {guild} should be entitled");
    }
}
