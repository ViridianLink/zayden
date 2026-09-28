//! The LISTEN/NOTIFY bus: the channels the database raises, the tables that
//! raise them, and what the listener does when its connection drops.

use std::time::Duration;

use sqlx::PgPool;
use tokio::sync::broadcast;
use tokio::time::{sleep, timeout};
use zayden_app::events::listener::EventListener;
use zayden_app::events::{AppEvent, Channel};

#[test]
fn channel_names_round_trip() {
    for channel in Channel::ALL {
        assert_eq!(Channel::from_name(channel.as_str()), Some(channel));
    }
    assert_eq!(Channel::from_name("not_a_channel"), None);
}

#[sqlx::test(migrations = "../migrations")]
async fn every_settings_table_notifies_config_changes(pool: PgPool) {
    let silent = sqlx::query_scalar!(
        r#"SELECT c.relname AS "relname!"
           FROM pg_class c
           JOIN pg_namespace n ON n.oid = c.relnamespace
           WHERE n.nspname = 'public'
             AND c.relkind = 'r'
             AND c.relname LIKE '%\_settings'
             AND NOT EXISTS (
                 SELECT 1 FROM pg_trigger t
                 JOIN pg_proc p ON p.oid = t.tgfoid
                 WHERE t.tgrelid = c.oid AND p.proname = 'notify_config_changed'
             )
           ORDER BY c.relname"#
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert!(
        silent.is_empty(),
        "settings tables without a config_changed trigger (add one with \
         `SELECT attach_config_notify('<table>')`): {silent:?}"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn every_channel_the_database_raises_is_one_the_listener_hears(pool: PgPool) {
    let sources = sqlx::query_scalar!(
        r#"SELECT p.prosrc AS "prosrc!"
           FROM pg_proc p
           JOIN pg_namespace n ON n.oid = p.pronamespace
           WHERE n.nspname = 'public' AND p.prosrc LIKE '%pg_notify(%'"#
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_ne!(sources.len(), 0, "no function raises a notification");

    for source in &sources {
        for call in source.split("pg_notify('").skip(1) {
            let name = call.split('\'').next().unwrap();
            assert!(
                Channel::from_name(name).is_some(),
                "the database raises `{name}`, which Channel does not list"
            );
        }
    }
}

async fn listener_pid(pool: &PgPool) -> sqlx::Result<Option<i32>> {
    for _ in 0..50 {
        let pid = sqlx::query_scalar!(
            "SELECT pid FROM pg_stat_activity
             WHERE datname = current_database()
               AND pid <> pg_backend_pid()
               AND query LIKE 'LISTEN%'"
        )
        .fetch_optional(pool)
        .await?
        .flatten();

        if pid.is_some() {
            return Ok(pid);
        }
        sleep(Duration::from_millis(100)).await;
    }

    Ok(None)
}

async fn next_event(rx: &mut broadcast::Receiver<AppEvent>) -> Option<AppEvent> {
    timeout(Duration::from_secs(15), rx.recv()).await.ok()?.ok()
}

#[sqlx::test(migrations = "../migrations")]
async fn a_dropped_connection_reconnects_and_asks_for_a_resync(pool: PgPool) {
    let (tx, mut rx) = broadcast::channel(16);
    EventListener::spawn(pool.clone(), tx);

    let pid =
        listener_pid(&pool).await.unwrap().expect("the listener never subscribed");
    let terminated = sqlx::query_scalar!("SELECT pg_terminate_backend($1)", pid)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(terminated, Some(true));

    assert!(
        matches!(next_event(&mut rx).await, Some(AppEvent::Resync)),
        "notifications sent while disconnected are lost, so caches must resync"
    );

    Channel::ConfigChanged.notify(&pool, "42").await.unwrap();

    assert!(
        matches!(next_event(&mut rx).await, Some(AppEvent::ConfigChanged(42))),
        "the new connection must be listening again"
    );
}
