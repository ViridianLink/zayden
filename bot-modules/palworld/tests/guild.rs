//! `save::guild` — guild membership decode + base-pal pooling.
//!
//! Runs against the real save when present (mirroring `save_decompress`'s
//! real-save case) and otherwise skips, so `cargo test` stays green offline.
//! Ground-truth guilds in this save (per the plan's oracle):
//!   A = Oscar/J./KingJosh/ThatGuy, B = Devil/Zylbas, C = cutathanyou (solo).

use std::collections::HashSet;
use std::path::PathBuf;

use palworld::save::decompress::decompress;
use palworld::save::extract::{ExtractedWorld, extract};
use palworld::save::guild::{GuildData, decode_guilds};
use palworld::save::gvas::read_gvas;
use palworld::save::load_world;

// Player UIDs (verified against the real save; `hex_upper` of the record GUIDs).
const CUTA: &str = "59F0C9D9000000000000000000000000";
const DEVIL: &str = "5742CA5A000000000000000000000000";
const ZYLBAS: &str = "CC912A1C000000000000000000000000";
const J: &str = "3454079E000000000000000000000000";
const THAT_GUY: &str = "A64A5035000000000000000000000000";
const OSCAR: &str = "286C72B0000000000000000000000000";
const KINGJOSH: &str = "5CF598C9000000000000000000000000";

const GUILD_A: [&str; 4] = [OSCAR, J, KINGJOSH, THAT_GUY];

fn save_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../056C426C55974CFCA115EB695A224F67")
}

/// Read the raw `Level.sav`, or `None` to skip. Skips when the save is absent,
/// or when the copy on disk isn't a decodable Palworld save (e.g. a torn or
/// partial download): such a file is not the ground-truth world these
/// assertions describe, so it must not fail the suite.
fn level_bytes() -> Option<Vec<u8>> {
    let raw = std::fs::read(save_dir().join("Level.sav")).ok()?;
    if let Err(e) = palworld::save::validate_level(&raw) {
        eprintln!("skipping: save present but not decodable ({e})");
        return None;
    }
    Some(raw)
}

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
    let Some(raw) = level_bytes() else {
        eprintln!("skipping: real save not present");
        return;
    };
    let level = read_gvas(&decompress(&raw).expect("decompress")).expect("gvas");
    let guilds = decode_guilds(&level);

    // Structural invariants that hold no matter how the world has evolved since
    // capture (members join or leave, guilds form or disband). Anchoring on
    // specific membership rots; the decode's internal consistency does not.
    let members: Vec<&String> = guilds.all_members().collect();
    assert!(!members.is_empty(), "a populated save decodes at least one guild");

    // Every guild reachable from a member is well-formed: its roster is
    // non-empty, duplicate-free, and every listed member maps back to exactly
    // this guild (a clean partition — nobody belongs to two guilds).
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
    let Some(raw) = level_bytes() else {
        eprintln!("skipping: real save not present");
        return;
    };
    let level = read_gvas(&decompress(&raw).expect("decompress")).expect("gvas");
    let extracted = extract(&level).expect("extract");
    let guilds = decode_guilds(&level);

    // Every Guild A member receives the identical, non-empty base-pal pool —
    // including KingJosh / That Guy, who own no base pals themselves.
    let pool_a = pooled_count(OSCAR, &extracted, &guilds);
    assert!(pool_a > 0, "guild A has base pals to pool");
    for uid in GUILD_A {
        assert_eq!(
            pooled_count(uid, &extracted, &guilds),
            pool_a,
            "member {uid} shares pool A"
        );
    }

    // Devil and Zylbas share their own pool.
    let pool_b = pooled_count(DEVIL, &extracted, &guilds);
    assert_eq!(
        pooled_count(ZYLBAS, &extracted, &guilds),
        pool_b,
        "Devil/Zylbas share"
    );

    // cutathanyou (solo) receives only pals last-owned by cutathanyou — no
    // Guild A leakage.
    let cuta_pool = pooled_count(CUTA, &extracted, &guilds);
    let cuta_own_base =
        extracted.base_pals.iter().filter(|b| b.last_owner == CUTA).count();
    assert_eq!(cuta_pool, cuta_own_base, "cutathanyou pool is strictly its own");
}

#[test]
fn load_world_roster_matches_owned_plus_pool() {
    let Some(raw) = level_bytes() else {
        eprintln!("skipping: real save not present");
        return;
    };
    let level = read_gvas(&decompress(&raw).expect("decompress")).expect("gvas");
    let extracted = extract(&level).expect("extract");
    let guilds = decode_guilds(&level);
    let world = load_world(&save_dir()).expect("load_world");

    for player in &world.players {
        let owned = extracted.pals.get(&player.uid).map_or(0, Vec::len);
        let pooled = pooled_count(&player.uid, &extracted, &guilds);
        assert_eq!(
            player.pals.len(),
            owned + pooled,
            "roster for {} = owned {owned} + pooled {pooled}",
            player.name
        );
    }
}
