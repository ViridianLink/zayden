//! The claim is what guarantees a post is announced once. It marks rows in the
//! same statement that selects them, so the failure mode of a crash mid-batch
//! is a dropped announcement rather than a repeated one — and two bot
//! processes polling at once cannot both take the same post.
//!
//! The rest of this file pins the multi-tenant rules: a guild only ever
//! receives posts from the campaign it authorised, and a connection without a
//! channel — or with a revoked grant — is not a subscriber.

use patreon::model::PatreonPost;
use patreon::store::{
    PatreonAnnounceRow,
    PatreonCampaignRow,
    PatreonConnection,
    claim_pending,
    insert_post,
    is_subscribed,
    webhook_secrets,
};
use sqlx::PgPool;

fn post(id: &str, campaign: &str, is_public: bool) -> PatreonPost {
    PatreonPost {
        id: id.to_owned(),
        campaign_id: campaign.to_owned(),
        title: Some(format!("Post {id}")),
        url: format!("https://patreon.test/{id}"),
        content_html: Some("<p>body</p>".to_owned()),
        is_public,
        published_at: timestamp("2026-09-01T00:00:00Z"),
    }
}

#[sqlx::test(migrations = "../../migrations", fixtures("patreon"))]
async fn the_claim_takes_only_unannounced_posts(pool: PgPool) {
    let claimed = claim_pending(&pool, 10).await.unwrap();

    let ids: Vec<&str> = claimed.iter().map(|p| p.post_id.as_str()).collect();
    assert_eq!(ids, ["p-public", "p-patrons"]);
}

#[sqlx::test(migrations = "../../migrations", fixtures("patreon"))]
async fn a_claimed_post_is_not_claimed_again(pool: PgPool) {
    assert_eq!(claim_pending(&pool, 10).await.unwrap().len(), 2);
    assert!(claim_pending(&pool, 10).await.unwrap().is_empty());
}

#[sqlx::test(migrations = "../../migrations", fixtures("patreon"))]
async fn the_claim_respects_its_batch_limit(pool: PgPool) {
    assert_eq!(claim_pending(&pool, 1).await.unwrap().len(), 1);
    assert_eq!(claim_pending(&pool, 1).await.unwrap().len(), 1);
    assert!(claim_pending(&pool, 1).await.unwrap().is_empty());
}

/// One creator, two servers: both get the public post, only the guild that
/// asked for everything gets the patrons-only one.
#[sqlx::test(migrations = "../../migrations", fixtures("patreon"))]
async fn a_public_only_guild_is_skipped_for_a_patrons_post(pool: PgPool) {
    let mut public: Vec<i64> = PatreonAnnounceRow::for_post(&pool, "100", true)
        .await
        .unwrap()
        .iter()
        .map(|row| row.guild_id)
        .collect();
    public.sort_unstable();

    let patrons: Vec<i64> = PatreonAnnounceRow::for_post(&pool, "100", false)
        .await
        .unwrap()
        .iter()
        .map(|row| row.guild_id)
        .collect();

    assert_eq!(public, [1, 2]);
    assert_eq!(patrons, [1]);
}

/// The whole multi-tenant guarantee in one assertion: a guild reaches a
/// campaign only through its own grant, so campaign 100's posts can never land
/// in a server connected to campaign 300.
#[sqlx::test(migrations = "../../migrations", fixtures("patreon"))]
async fn a_guild_never_receives_a_campaign_it_did_not_authorise(pool: PgPool) {
    let recipients = PatreonAnnounceRow::for_post(&pool, "100", true).await.unwrap();

    assert!(
        recipients.iter().all(|row| row.guild_id != 3),
        "guild 3 is connected to campaign 300"
    );
}

/// A revoked grant stops delivery without anyone having to remove the channel.
#[sqlx::test(migrations = "../../migrations", fixtures("patreon"))]
async fn a_disabled_connection_receives_nothing(pool: PgPool) {
    let recipients = PatreonAnnounceRow::for_post(&pool, "400", true).await.unwrap();

    assert!(recipients.is_empty(), "{recipients:?}");
}

#[sqlx::test(migrations = "../../migrations", fixtures("patreon"))]
async fn only_connected_campaigns_with_a_channel_are_polled(pool: PgPool) {
    let pollable = PatreonConnection::pollable(&pool).await.unwrap();

    let campaigns: Vec<&str> =
        pollable.iter().map(|c| c.campaign_id.as_str()).collect();

    assert_eq!(campaigns, ["100"], "300 has no channel and 400's grant is disabled");
}

/// Two guilds on one creator share a cursor, so the campaign is fetched once
/// per pass rather than once per guild.
#[sqlx::test(migrations = "../../migrations", fixtures("patreon"))]
async fn a_shared_campaign_is_polled_once(pool: PgPool) {
    let pollable = PatreonConnection::pollable(&pool).await.unwrap();

    assert_eq!(pollable.len(), 1);
    assert_eq!(pollable.first().map(|c| c.guild_id), Some(1));
}

#[sqlx::test(migrations = "../../migrations", fixtures("patreon"))]
async fn subscription_gates_the_webhook(pool: PgPool) {
    assert!(is_subscribed(&pool, "100").await.unwrap());
    assert!(!is_subscribed(&pool, "300").await.unwrap(), "no channel");
    assert!(!is_subscribed(&pool, "400").await.unwrap(), "grant disabled");
    assert!(!is_subscribed(&pool, "999").await.unwrap());
}

/// Each guild registers its own hook, so a campaign can have several secrets
/// and an inbound delivery has to be checked against all of them.
#[sqlx::test(migrations = "../../migrations", fixtures("patreon"))]
async fn every_live_secret_for_a_campaign_is_returned(pool: PgPool) {
    let mut secrets = webhook_secrets(&pool, "100").await.unwrap();
    secrets.sort();

    assert_eq!(secrets, ["secret-1", "secret-2"]);
    assert!(
        webhook_secrets(&pool, "400").await.unwrap().is_empty(),
        "a disabled grant's secret is not accepted"
    );
}

/// The same post arriving from both the poll and the webhook must not produce
/// two rows, and therefore not two announcements.
#[sqlx::test(migrations = "../../migrations", fixtures("patreon"))]
async fn inserting_a_post_twice_is_a_no_op(pool: PgPool) {
    let post = post("p-new", "100", true);

    assert!(insert_post(&pool, &post, false).await.unwrap());
    assert!(!insert_post(&pool, &post, false).await.unwrap());

    let claimed = claim_pending(&pool, 10).await.unwrap();
    assert_eq!(claimed.iter().filter(|p| p.post_id == "p-new").count(), 1);
}

/// Seeding is what stops connecting a campaign from replaying its entire back
/// catalogue into the channel.
#[sqlx::test(migrations = "../../migrations", fixtures("patreon"))]
async fn a_post_inserted_as_announced_is_never_claimed(pool: PgPool) {
    assert!(insert_post(&pool, &post("p-seeded", "100", true), true).await.unwrap());

    let claimed = claim_pending(&pool, 10).await.unwrap();
    assert!(claimed.iter().all(|p| p.post_id != "p-seeded"), "{claimed:?}");
}

#[sqlx::test(migrations = "../../migrations", fixtures("patreon"))]
async fn a_successful_poll_advances_the_cursor_and_seeds(pool: PgPool) {
    PatreonCampaignRow::ensure(&pool, "300").await.unwrap();
    PatreonCampaignRow::record_success(&pool, "300", Some("cursor-9"))
        .await
        .unwrap();

    let row = campaign(&pool, "300").await;
    assert_eq!(row.next_cursor.as_deref(), Some("cursor-9"));
    assert!(row.is_seeded(), "the first successful poll seeds the campaign");
}

/// `seeded_at` is the boundary between absorbing and announcing, so a later
/// poll must not move it.
#[sqlx::test(migrations = "../../migrations", fixtures("patreon"))]
async fn a_later_poll_does_not_reseed(pool: PgPool) {
    let before = campaign(&pool, "100").await.seeded_at;

    PatreonCampaignRow::record_success(&pool, "100", Some("cursor-2"))
        .await
        .unwrap();

    assert_eq!(campaign(&pool, "100").await.seeded_at, before);
}

#[sqlx::test(migrations = "../../migrations", fixtures("patreon"))]
async fn failures_accumulate_and_a_success_clears_them(pool: PgPool) {
    assert_eq!(PatreonCampaignRow::record_failure(&pool, "100").await.unwrap(), 1);
    assert_eq!(PatreonCampaignRow::record_failure(&pool, "100").await.unwrap(), 2);

    PatreonCampaignRow::record_success(&pool, "100", None).await.unwrap();

    assert_eq!(campaign(&pool, "100").await.consecutive_failures, 0);
}

#[sqlx::test(migrations = "../../migrations", fixtures("patreon"))]
async fn setting_a_channel_does_not_touch_the_connection(pool: PgPool) {
    PatreonAnnounceRow::upsert(&pool, 1, 9999, true).await.unwrap();

    let row = PatreonAnnounceRow::select(&pool, 1).await.unwrap().unwrap();
    assert_eq!(row.channel_id, 9999);
    assert!(row.public_only);

    let connection = PatreonConnection::select(&pool, 1).await.unwrap().unwrap();
    assert_eq!(connection.campaign_id, "100");
}

#[sqlx::test(migrations = "../../migrations", fixtures("patreon"))]
async fn disable_reports_whether_anything_was_removed(pool: PgPool) {
    assert!(PatreonAnnounceRow::delete(&pool, 1).await.unwrap());
    assert!(!PatreonAnnounceRow::delete(&pool, 1).await.unwrap());
}

/// Disconnecting removes the grant, which takes the guild out of every path at
/// once: no poll, no webhook, no announcement.
#[sqlx::test(migrations = "../../migrations", fixtures("patreon"))]
async fn deleting_a_connection_unsubscribes_the_guild(pool: PgPool) {
    assert!(PatreonConnection::delete(&pool, 2).await.unwrap());

    let recipients = PatreonAnnounceRow::for_post(&pool, "100", true).await.unwrap();
    assert_eq!(recipients.iter().map(|row| row.guild_id).collect::<Vec<_>>(), [1]);

    assert_eq!(webhook_secrets(&pool, "100").await.unwrap(), ["secret-1"]);
}

#[expect(
    clippy::expect_used,
    reason = "a free helper sits outside the #[test] items clippy.toml exempts"
)]
fn timestamp(raw: &str) -> jiff::Timestamp {
    raw.parse().expect("a fixture timestamp parses")
}

#[expect(
    clippy::expect_used,
    reason = "a free helper sits outside the #[test] items clippy.toml exempts"
)]
async fn campaign(pool: &PgPool, id: &str) -> PatreonCampaignRow {
    PatreonCampaignRow::select(pool, id)
        .await
        .expect("the campaign lookup runs")
        .expect("the campaign row exists")
}
