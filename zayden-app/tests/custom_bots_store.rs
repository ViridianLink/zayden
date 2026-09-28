use secrecy::{ExposeSecret, SecretString};
use sqlx::PgPool;
use sqlx::postgres::PgListener;
use zayden_app::custom_bots::{
    CustomBotError,
    CustomBotStatus,
    CustomBotStore,
    Keyring,
    NewCustomBot,
};

const KEY: &str = "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=";
const LLAMA: i64 = 200;
const OTHER: i64 = 300;
const GUILD: i64 = 10;
const SECOND_GUILD: i64 = 20;
const ADMIN: i64 = 1;
const TOKEN: &str = "not.a-real.token";

#[expect(
    clippy::unwrap_used,
    reason = "a free helper sits outside the #[test] items clippy.toml exempts"
)]
async fn register(pool: &PgPool, application_id: i64) {
    let keys = Keyring::parse(&format!("1:{KEY}"), "1").unwrap();
    let sealed =
        keys.seal(application_id, &SecretString::from(TOKEN.to_owned())).unwrap();
    let bot = NewCustomBot {
        application_id,
        bot_user_id: application_id,
        name: "LlamaBot",
        avatar: None,
        public_key: "public",
        registered_by: ADMIN,
    };
    CustomBotStore::upsert(pool, &bot, &sealed).await.unwrap();
}

#[sqlx::test(migrations = "../migrations")]
async fn a_stored_token_round_trips(pool: PgPool) {
    register(&pool, LLAMA).await;

    let keys = Keyring::parse(&format!("1:{KEY}"), "1").unwrap();
    let sealed = CustomBotStore::sealed_token(&pool, LLAMA).await.unwrap().unwrap();

    assert_eq!(keys.open(LLAMA, &sealed).unwrap().expose_secret(), TOKEN);
}

#[sqlx::test(migrations = "../migrations")]
async fn one_bot_can_serve_several_guilds(pool: PgPool) {
    register(&pool, LLAMA).await;

    CustomBotStore::attach(&pool, GUILD, LLAMA, ADMIN).await.unwrap();
    CustomBotStore::attach(&pool, SECOND_GUILD, LLAMA, ADMIN).await.unwrap();

    assert_eq!(CustomBotStore::guilds(&pool, LLAMA).await.unwrap(), [
        GUILD,
        SECOND_GUILD
    ]);
}

#[sqlx::test(migrations = "../migrations")]
async fn attaching_the_same_bot_twice_is_idempotent(pool: PgPool) {
    register(&pool, LLAMA).await;

    CustomBotStore::attach(&pool, GUILD, LLAMA, ADMIN).await.unwrap();
    CustomBotStore::attach(&pool, GUILD, LLAMA, ADMIN).await.unwrap();

    assert_eq!(CustomBotStore::guilds(&pool, LLAMA).await.unwrap(), [GUILD]);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_guild_takes_only_one_custom_bot(pool: PgPool) {
    register(&pool, LLAMA).await;
    register(&pool, OTHER).await;
    CustomBotStore::attach(&pool, GUILD, LLAMA, ADMIN).await.unwrap();

    let refused = CustomBotStore::attach(&pool, GUILD, OTHER, ADMIN).await;

    assert!(matches!(
        refused,
        Err(CustomBotError::GuildTaken { guild_id: GUILD, application_id: LLAMA })
    ));
    assert_eq!(
        CustomBotStore::guilds(&pool, OTHER).await.unwrap(),
        Vec::<i64>::new()
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn purging_a_guild_detaches_its_bot(pool: PgPool) {
    register(&pool, LLAMA).await;
    CustomBotStore::attach(&pool, GUILD, LLAMA, ADMIN).await.unwrap();

    sqlx::query!("DELETE FROM guilds WHERE id = $1", GUILD)
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(
        CustomBotStore::guilds(&pool, LLAMA).await.unwrap(),
        Vec::<i64>::new()
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn detaching_reports_whether_anything_was_attached(pool: PgPool) {
    register(&pool, LLAMA).await;
    CustomBotStore::attach(&pool, GUILD, LLAMA, ADMIN).await.unwrap();

    assert!(CustomBotStore::detach(&pool, GUILD).await.unwrap());
    assert!(!CustomBotStore::detach(&pool, GUILD).await.unwrap());
}

#[sqlx::test(migrations = "../migrations")]
async fn change_notifications_carry_ids_and_never_the_token(pool: PgPool) {
    register(&pool, LLAMA).await;
    CustomBotStore::attach(&pool, GUILD, LLAMA, ADMIN).await.unwrap();

    let mut listener = PgListener::connect_with(&pool).await.unwrap();
    listener.listen_all(["custom_bots_changed", "serving_changed"]).await.unwrap();

    CustomBotStore::set_status(&pool, LLAMA, CustomBotStatus::Revoked)
        .await
        .unwrap();

    let mut received = Vec::new();
    for _ in 0..2 {
        let notification = listener.recv().await.unwrap();
        assert!(!notification.payload().contains(TOKEN));
        received.push((
            notification.channel().to_owned(),
            notification.payload().to_owned(),
        ));
    }
    received.sort();

    assert_eq!(received, [
        ("custom_bots_changed".to_owned(), LLAMA.to_string()),
        ("serving_changed".to_owned(), GUILD.to_string()),
    ]);
}
