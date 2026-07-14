//! Unit coverage for the user-upload feature (Milestone 9).
//!
//! The `SaveUpload` cooldown/expiry logic is pure and runs everywhere. The
//! `load_world` round-trip needs the real save and is gated on `PALWORLD_SAVE=1`
//! (mirroring `tests/save_world.rs`); it proves an uploaded, `Level.sav`-only
//! world (no `Players/` directory) parses cleanly.

use std::path::PathBuf;

use jiff::{SignedDuration, Timestamp};
use jiff_sqlx::ToSqlx;
use palworld::save::load_world;
use palworld::upload::{SaveUpload, UPLOAD_COOLDOWN};

fn upload(uploaded: Timestamp, expires: Timestamp) -> SaveUpload {
    SaveUpload {
        discord_id: 1,
        file_path: "uploads/1/Level.sav".to_string(),
        uploaded_at: uploaded.to_sqlx(),
        expires_at: expires.to_sqlx(),
    }
}

#[test]
fn fresh_upload_is_within_cooldown_and_not_expired() {
    let now = Timestamp::now();
    let expires = now.checked_add(SignedDuration::from_hours(24 * 7)).unwrap();
    let u = upload(now, expires);

    let remaining = u.cooldown_remaining().expect("fresh upload is cooling down");
    assert!(remaining > SignedDuration::ZERO);
    assert!(remaining <= UPLOAD_COOLDOWN);
    assert!(!u.is_expired());
}

#[test]
fn cooldown_lifts_after_the_window() {
    let now = Timestamp::now();
    let uploaded = now
        .checked_sub(UPLOAD_COOLDOWN)
        .unwrap()
        .checked_sub(SignedDuration::from_secs(1))
        .unwrap();
    let expires = now.checked_add(SignedDuration::from_hours(24 * 7)).unwrap();
    let u = upload(uploaded, expires);

    assert!(u.cooldown_remaining().is_none(), "cooldown has elapsed");
    assert!(!u.is_expired(), "still within the 1-week TTL");
}

#[test]
fn past_expiry_is_expired() {
    let now = Timestamp::now();
    let uploaded = now.checked_sub(SignedDuration::from_hours(24 * 8)).unwrap();
    let expires = now.checked_sub(SignedDuration::from_secs(1)).unwrap();
    let u = upload(uploaded, expires);

    assert!(u.is_expired(), "TTL elapsed");
}

fn real_save_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../056C426C55974CFCA115EB695A224F67")
}

fn save_enabled() -> bool {
    std::env::var("PALWORLD_SAVE").is_ok()
        && std::fs::read(real_save_dir().join("Level.sav"))
            .is_ok_and(|raw| palworld::save::validate_level(&raw).is_ok())
}

/// An uploaded world is `Level.sav`-only — no `Players/` directory. `load_world`
/// must treat the absent directory as an empty UID set and still parse.
#[test]
fn uploaded_level_only_world_parses_without_players_dir() {
    if !save_enabled() {
        eprintln!("skipping: set PALWORLD_SAVE=1 with the real save present");
        return;
    }

    let raw =
        std::fs::read(real_save_dir().join("Level.sav")).expect("read Level.sav");

    let dir = std::env::temp_dir()
        .join(format!("palworld_upload_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mk temp dir");
    std::fs::write(dir.join("Level.sav"), &raw).expect("write Level.sav");
    assert!(!dir.join("Players").exists(), "no Players/ dir, as with a real upload");

    let world = load_world(&dir);
    std::fs::remove_dir_all(&dir).ok();

    let world = world.expect("Level.sav-only world parses");
    assert!(!world.players.is_empty(), "players decode from Level.sav alone");
}
