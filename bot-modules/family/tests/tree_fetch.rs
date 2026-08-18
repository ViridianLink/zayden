//! Component-walk coverage for `RawGraph::fetch`.
//!
//! The rendered tree is only as correct as this walk: anything it misses shows
//! up as a missing box, and anything it over-collects shows up as a stranger.

use family::TreeQuota;
use family::tree::RawGraph;
use serenity::all::{GuildId, UserId};
use sqlx::PgPool;

const GUILD: GuildId = GuildId::new(1);
const OTHER_GUILD: GuildId = GuildId::new(2);

macro_rules! ids {
    ($graph:expr) => {{
        let mut ids: Vec<i64> = $graph.people.iter().map(|p| p.id).collect();
        ids.sort_unstable();
        ids
    }};
}

#[sqlx::test(migrations = "../../migrations", fixtures("family_graph"))]
async fn walks_the_whole_component_in_every_direction(pool: PgPool) {
    let graph =
        RawGraph::fetch(&pool, GUILD, UserId::new(10), TreeQuota::FREE)
            .await
            .expect("fetch should succeed");

    // 15 is the partner's parent: reachable only by going sideways to 11 and
    // then upwards. The walk this replaced stopped at partners.
    assert_eq!(ids!(graph), vec![10, 11, 12, 13, 14, 15]);
    assert!(!graph.truncated);
}

#[sqlx::test(migrations = "../../migrations", fixtures("family_graph"))]
async fn a_disjoint_family_in_the_same_guild_is_excluded(pool: PgPool) {
    let graph =
        RawGraph::fetch(&pool, GUILD, UserId::new(10), TreeQuota::FREE)
            .await
            .expect("fetch should succeed");

    for stranger in [20, 21, 30, 31] {
        assert!(
            !ids!(graph).contains(&stranger),
            "user {stranger} is in a different component",
        );
    }
}

#[sqlx::test(migrations = "../../migrations", fixtures("family_graph"))]
async fn the_component_is_scoped_to_one_guild(pool: PgPool) {
    let here = RawGraph::fetch(&pool, GUILD, UserId::new(10), TreeQuota::FREE)
        .await
        .expect("fetch should succeed");
    let there =
        RawGraph::fetch(&pool, OTHER_GUILD, UserId::new(10), TreeQuota::FREE)
            .await
            .expect("fetch should succeed");

    assert!(!ids!(here).contains(&40), "guild 2's partner must not leak");
    assert_eq!(ids!(there), vec![10, 40]);
}

/// The schema only forbids self-loops, so `A parent of B, B parent of A` is
/// storable. `UNION` dedupes the recursive term on id, which is what stops
/// this from spinning forever.
#[sqlx::test(migrations = "../../migrations", fixtures("family_graph"))]
async fn a_parent_cycle_terminates(pool: PgPool) {
    let graph =
        RawGraph::fetch(&pool, GUILD, UserId::new(30), TreeQuota::FREE)
            .await
            .expect("a cycle must not hang the walk");

    assert_eq!(ids!(graph), vec![30, 31]);
}

#[sqlx::test(migrations = "../../migrations", fixtures("family_graph"))]
async fn edges_are_returned_only_for_the_component(pool: PgPool) {
    let graph =
        RawGraph::fetch(&pool, GUILD, UserId::new(10), TreeQuota::FREE)
            .await
            .expect("fetch should succeed");

    assert_eq!(graph.partners, vec![(10, 11)]);
    assert_eq!(graph.parents, vec![(10, 12), (13, 10), (14, 13), (15, 11)]);
}

/// The fetch limit is a blast-radius guard. When it binds, the caller has to
/// know the component was cut short rather than genuinely small.
#[sqlx::test(migrations = "../../migrations", fixtures("family_graph"))]
async fn hitting_the_fetch_limit_is_reported(pool: PgPool) {
    let tight = TreeQuota { fetch_limit: 3, ..TreeQuota::FREE };

    let graph = RawGraph::fetch(&pool, GUILD, UserId::new(10), tight)
        .await
        .expect("fetch should succeed");

    assert_eq!(graph.len(), 3);
    assert!(graph.truncated, "a truncated component must say so");
}

#[sqlx::test(migrations = "../../migrations", fixtures("family_graph"))]
async fn a_user_with_no_family_yields_an_empty_graph(pool: PgPool) {
    let graph =
        RawGraph::fetch(&pool, GUILD, UserId::new(9_999), TreeQuota::FREE)
            .await
            .expect("fetch should succeed");

    assert!(graph.is_empty());
}
