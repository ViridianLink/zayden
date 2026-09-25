//! The claim marks rows in the statement that selects them, so a video is
//! announced at most once even with two bot processes polling. The rest pins
//! the multi-tenant rules: a guild only hears from the channel it connected,
//! and several guilds on one channel share one hub subscription.

use sqlx::PgPool;
use youtube::model::{OwnChannel, YoutubeVideo};
use youtube::store::{
    YoutubeAnnounceRow,
    YoutubeChannelRow,
    YoutubeConnection,
    claim_pending,
    insert_video,
    is_subscribed,
};

fn video(id: &str, channel: &str) -> YoutubeVideo {
    YoutubeVideo {
        id: id.to_owned(),
        channel_id: channel.to_owned(),
        title: format!("Video {id}"),
        published_at: jiff::Timestamp::now(),
    }
}

fn channel(id: &str) -> OwnChannel {
    OwnChannel {
        id: id.to_owned(),
        title: format!("Channel {id}"),
        uploads_playlist_id: format!("UU{id}"),
    }
}

#[sqlx::test(migrations = "../../migrations", fixtures("youtube"))]
async fn the_claim_takes_only_unannounced_videos_oldest_first(pool: PgPool) {
    let claimed = claim_pending(&pool, 10).await.unwrap();

    let ids: Vec<&str> = claimed.iter().map(|v| v.video_id.as_str()).collect();
    assert_eq!(ids, ["v-first", "v-second"]);
    assert_eq!(claimed[0].channel_title, "Shared Creator");
}

#[sqlx::test(migrations = "../../migrations", fixtures("youtube"))]
async fn a_claimed_video_is_not_claimed_again(pool: PgPool) {
    assert_eq!(claim_pending(&pool, 10).await.unwrap().len(), 2);
    assert!(claim_pending(&pool, 10).await.unwrap().is_empty());
}

#[sqlx::test(migrations = "../../migrations", fixtures("youtube"))]
async fn a_video_inserted_as_announced_is_never_claimed(pool: PgPool) {
    assert!(insert_video(&pool, &video("v-seeded", "UCother"), true).await.unwrap());

    let claimed = claim_pending(&pool, 10).await.unwrap();
    assert!(claimed.iter().all(|v| v.video_id != "v-seeded"), "{claimed:?}");
}

#[sqlx::test(migrations = "../../migrations", fixtures("youtube"))]
async fn a_video_is_stored_once(pool: PgPool) {
    assert!(insert_video(&pool, &video("v-new", "UCother"), false).await.unwrap());
    assert!(!insert_video(&pool, &video("v-new", "UCother"), false).await.unwrap());
}

/// Guild 2 shares the channel but has no announce channel, so only guild 1
/// hears about it; guild 3 follows another channel entirely.
#[sqlx::test(migrations = "../../migrations", fixtures("youtube"))]
async fn a_video_reaches_only_guilds_following_its_channel(pool: PgPool) {
    let shared: Vec<i64> = YoutubeAnnounceRow::for_channel(&pool, "UCshared")
        .await
        .unwrap()
        .iter()
        .map(|row| row.guild_id)
        .collect();
    let other: Vec<i64> = YoutubeAnnounceRow::for_channel(&pool, "UCother")
        .await
        .unwrap()
        .iter()
        .map(|row| row.guild_id)
        .collect();

    assert_eq!(shared, [1]);
    assert_eq!(other, [3]);
}

#[sqlx::test(migrations = "../../migrations", fixtures("youtube"))]
async fn only_channels_with_an_announcing_guild_are_polled(pool: PgPool) {
    let polled: Vec<String> = YoutubeChannelRow::pollable(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.channel_id)
        .collect();

    assert_eq!(polled, ["UCother", "UCshared"]);
    assert!(is_subscribed(&pool, "UCshared").await.unwrap());
    assert!(!is_subscribed(&pool, "UCorphan").await.unwrap());
}

/// `UCshared`'s lease has days left and `UCorphan` has no connection to renew
/// for.
#[sqlx::test(migrations = "../../migrations", fixtures("youtube"))]
async fn only_leases_near_expiry_on_connected_channels_are_renewed(pool: PgPool) {
    let due: Vec<String> = YoutubeChannelRow::lease_due(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.channel_id)
        .collect();

    assert_eq!(due, ["UCother"]);
}

#[sqlx::test(migrations = "../../migrations", fixtures("youtube"))]
async fn a_second_guild_on_a_channel_reuses_its_hub_secret(pool: PgPool) {
    sqlx::query!("INSERT INTO guilds (id) VALUES (4)").execute(&pool).await.unwrap();

    let secret =
        YoutubeConnection::connect(&pool, 4, &channel("shared"), 9004, "fresh")
            .await
            .unwrap();
    assert_eq!(secret, "fresh", "a new channel takes the offered secret");

    let reused = YoutubeConnection::connect(
        &pool,
        4,
        &OwnChannel {
            id: "UCshared".to_owned(),
            title: "Renamed".to_owned(),
            uploads_playlist_id: "UUshared".to_owned(),
        },
        9004,
        "ignored",
    )
    .await
    .unwrap();

    assert_eq!(reused, "secret-shared");

    let connection = YoutubeConnection::select(&pool, 4).await.unwrap().unwrap();
    assert_eq!(connection.channel_id, "UCshared");
    assert_eq!(connection.channel_title, "Renamed");
    assert!(connection.lease_active);
}

#[sqlx::test(migrations = "../../migrations", fixtures("youtube"))]
async fn the_hub_subscription_outlives_all_but_the_last_guild(pool: PgPool) {
    assert_eq!(
        YoutubeConnection::delete(&pool, 1).await.unwrap().as_deref(),
        Some("UCshared")
    );
    assert!(
        YoutubeConnection::channel_has_connections(&pool, "UCshared").await.unwrap()
    );

    YoutubeConnection::delete(&pool, 2).await.unwrap();
    assert!(
        !YoutubeConnection::channel_has_connections(&pool, "UCshared")
            .await
            .unwrap()
    );

    assert_eq!(YoutubeConnection::delete(&pool, 2).await.unwrap(), None);
}

#[sqlx::test(migrations = "../../migrations", fixtures("youtube"))]
async fn seeding_happens_once(pool: PgPool) {
    YoutubeChannelRow::record_success(&pool, "UCother").await.unwrap();
    let first = YoutubeChannelRow::select(&pool, "UCother").await.unwrap().unwrap();
    assert!(first.is_seeded());

    YoutubeChannelRow::record_success(&pool, "UCother").await.unwrap();
    let second = YoutubeChannelRow::select(&pool, "UCother").await.unwrap().unwrap();
    assert_eq!(first.seeded_at, second.seeded_at);
}

#[sqlx::test(migrations = "../../migrations", fixtures("youtube"))]
async fn failures_accumulate_and_a_success_clears_them(pool: PgPool) {
    assert_eq!(
        YoutubeChannelRow::record_failure(&pool, "UCother").await.unwrap(),
        1
    );
    assert_eq!(
        YoutubeChannelRow::record_failure(&pool, "UCother").await.unwrap(),
        2
    );

    YoutubeChannelRow::record_success(&pool, "UCother").await.unwrap();

    assert_eq!(
        YoutubeChannelRow::record_failure(&pool, "UCother").await.unwrap(),
        1
    );
}

#[sqlx::test(migrations = "../../migrations", fixtures("youtube"))]
async fn a_confirmed_lease_is_recorded_and_can_be_cleared(pool: PgPool) {
    YoutubeChannelRow::set_lease(&pool, "UCorphan", 432_000).await.unwrap();
    sqlx::query!("INSERT INTO guilds (id) VALUES (5)").execute(&pool).await.unwrap();
    sqlx::query!(
        "INSERT INTO youtube_connections (guild_id, channel_id, connected_by)
         VALUES (5, 'UCorphan', 1)"
    )
    .execute(&pool)
    .await
    .unwrap();

    assert!(
        YoutubeConnection::select(&pool, 5).await.unwrap().unwrap().lease_active
    );

    YoutubeChannelRow::clear_lease(&pool, "UCorphan").await.unwrap();
    assert!(
        !YoutubeConnection::select(&pool, 5).await.unwrap().unwrap().lease_active
    );
}
