//! `save::guild` - guild membership decode + base-pal pooling.
//!
//! Runs against the committed `progressed-world` fixture, so these assertions
//! always execute.
//!
//! Membership drifts constantly as players join, leave and re-form guilds -
//! anchoring on a named player rots the moment they go solo, which is exactly
//! what happened to the earlier version of these assertions. Everything below
//! therefore picks its subjects out of the decode and asserts the *invariants*:
//! guilds partition the players cleanly, guildmates share one identical base-pal
//! pool, and a solo player's pool is strictly their own.

use std::collections::HashSet;

use palworld::save::dps;
use palworld::save::extract::{ExtractedWorld, extract};
use palworld::save::guild::{GuildData, decode_guilds};

pub mod common;
use common::progressed_world as save_dir;

/// Number of base pals a player receives once guild pooling is applied.
fn pooled_count(uid: &str, extracted: &ExtractedWorld, guilds: &GuildData) -> usize {
    extracted
        .base_pals
        .iter()
        .filter(|b| {
            guilds.guild_of(&b.last_owner).map_or_else(
                || b.last_owner == uid,
                |gid| guilds.members(gid).iter().any(|m| m == uid),
            )
        })
        .count()
}

#[test]
fn decodes_guilds_with_consistent_membership() {
    let level = common::progressed_gvas().expect("decode fixture Level.sav");
    let guilds = decode_guilds(level);

    // Structural invariants that hold no matter how the world has evolved since
    // capture (members join or leave, guilds form or disband). Anchoring on
    // specific membership rots; the decode's internal consistency does not.
    let members: Vec<&String> = guilds.all_members().collect();
    assert!(!members.is_empty(), "a populated save decodes at least one guild");

    // Every guild reachable from a member is well-formed: its roster is
    // non-empty, duplicate-free, and every listed member maps back to exactly
    // this guild (a clean partition - nobody belongs to two guilds).
    let guild_ids: HashSet<&str> =
        members.iter().filter_map(|m| guilds.guild_of(m)).collect();
    for gid in guild_ids {
        let roster = guilds.members(gid);
        assert!(!roster.is_empty(), "guild {gid} has members");

        let mut seen: HashSet<&String> = HashSet::new();
        for member in roster {
            assert_eq!(
                guilds.guild_of(member),
                Some(gid),
                "roster member {member} maps back to guild {gid}",
            );
            assert!(seen.insert(member), "guild {gid} lists {member} once");
        }
    }
}

#[test]
fn base_pals_pool_across_guild_members_only() {
    let level = common::progressed_gvas().expect("decode fixture Level.sav");
    let extracted = extract(level).expect("extract");
    let guilds = decode_guilds(level);

    // Group the decode's members by guild, so the subjects below are whoever
    // happens to be grouped together today.
    let mut by_guild: std::collections::BTreeMap<&str, Vec<&String>> =
        std::collections::BTreeMap::new();
    for member in guilds.all_members() {
        if let Some(gid) = guilds.guild_of(member) {
            by_guild.entry(gid).or_default().push(member);
        }
    }
    assert!(!by_guild.is_empty(), "a populated save decodes at least one guild");

    // Every member of a multi-player guild receives the identical base-pal pool,
    // including members who own no base pals themselves.
    let mut shared_pools = 0usize;
    for (gid, members) in by_guild.iter().filter(|(_, m)| m.len() > 1) {
        let Some(first) = members.first() else { continue };
        let expected = pooled_count(first, &extracted, &guilds);
        for uid in members {
            assert_eq!(
                pooled_count(uid, &extracted, &guilds),
                expected,
                "member {uid} shares guild {gid}'s pool",
            );
        }
        if expected > 0 {
            shared_pools += 1;
        }
    }
    assert!(shared_pools > 0, "at least one guild has base pals to pool");

    // A solo player receives only pals they last owned - no leakage from any
    // other guild's base camps.
    let mut solos = 0usize;
    for (_, members) in by_guild.iter().filter(|(_, m)| m.len() == 1) {
        let Some(uid) = members.first() else { continue };
        let own =
            extracted.base_pals.iter().filter(|b| &&b.last_owner == uid).count();
        assert_eq!(
            pooled_count(uid, &extracted, &guilds),
            own,
            "solo player {uid}'s pool is strictly their own",
        );
        solos += 1;
    }
    assert!(solos > 0, "the world has at least one solo guild to isolate");
}

/// A roster is exactly three sources added together and nothing else: the Pals
/// `Level.sav` files under the player, the Pals in their Dimensional Pal Storage,
/// and their share of the guild's base-camp pool.
#[test]
fn load_world_roster_matches_owned_plus_storage_plus_pool() {
    let level = common::progressed_gvas().expect("decode fixture Level.sav");
    let extracted = extract(level).expect("extract");
    let guilds = decode_guilds(level);
    let stored = dps::load_all(&save_dir());
    let world = common::progressed_world_roster().expect("load fixture world");

    for player in &world.players {
        let owned = extracted.pals.get(&player.uid).map_or(0, Vec::len);
        let storage = stored.get(&player.uid).map_or(0, Vec::len);
        let pooled = pooled_count(&player.uid, &extracted, &guilds);
        assert_eq!(
            player.pals.len(),
            owned + storage + pooled,
            "roster for {} = owned {owned} + storage {storage} + pooled {pooled}",
            player.name
        );
    }
}
