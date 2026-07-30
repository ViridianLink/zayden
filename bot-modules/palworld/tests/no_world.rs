//! What the shared-save read paths do before a world has landed on disk.
//!
//! On a container deploy the shared save directory is a relative path inside the
//! image with nothing mounted over it, so `Level.sav` does not exist until the
//! first Pelican refresh writes it. That window is normal, not a fault - but
//! every shared read funnels through `level_mtime`, which used to hand back the
//! raw `ENOENT` as [`PalworldError::Io`]. That turned a routine cold start into
//! `palworld: player name warm failed | error=io error: No such file or
//! directory (os error 2)` on every warm, and told users to "re-upload a fresh
//! Level.sav" for a world they never uploaded.
//!
//! Absence of the save is [`PalworldError::NoWorld`]. These tests pin that.

use std::path::PathBuf;
use std::{fs, io};

use palworld::client::{PalworldClient, SourceKey};
use palworld::error::PalworldError;

/// A path under the temp dir, unique per test and per process run.
fn scratch_path(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join(format!("palworld-no-world-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

/// A save directory that exists but holds no `Level.sav` - the state a fresh
/// container is in between start-up and the first successful refresh.
///
/// A macro rather than a function: the workspace denies `expect_used` and only
/// exempts test functions, so the call has to expand inside the `#[test]`.
macro_rules! empty_save_dir {
    ($tag:expr) => {{
        let dir = scratch_path($tag);
        fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }};
}

fn client(save_dir: Option<PathBuf>) -> PalworldClient {
    PalworldClient::new(
        reqwest::Client::new(),
        None,
        None,
        None,
        save_dir,
        PathBuf::from("palworld_uploads"),
        None,
    )
}

/// The exact production symptom: the cache warm runs before any save exists.
#[tokio::test]
async fn a_missing_level_save_is_no_world_not_an_io_error() {
    let dir = empty_save_dir!("names");
    let err = client(Some(dir.clone()))
        .player_names(SourceKey::Shared)
        .await
        .expect_err("no Level.sav means no names");

    let _ = fs::remove_dir_all(&dir);

    assert!(matches!(err, PalworldError::NoWorld), "expected NoWorld, got {err:?}");
}

/// A save directory that was never created at all resolves the same way - a
/// missing parent is still just "no world yet", not a broken deployment.
#[tokio::test]
async fn a_missing_save_directory_is_also_no_world() {
    let dir = scratch_path("absent");

    let err = client(Some(dir))
        .player_names(SourceKey::Shared)
        .await
        .expect_err("no directory means no names");

    assert!(matches!(err, PalworldError::NoWorld), "expected NoWorld, got {err:?}");
}

/// `roster` reads through the same helper, so it must not report a cold start as
/// a corrupt upload either.
#[tokio::test]
async fn the_roster_reports_no_world_before_the_first_refresh() {
    let dir = empty_save_dir!("roster");
    let err = client(Some(dir.clone()))
        .roster()
        .await
        .expect_err("no Level.sav means no roster");

    let _ = fs::remove_dir_all(&dir);

    assert!(matches!(err, PalworldError::NoWorld), "expected NoWorld, got {err:?}");
}

/// Absence is the only I/O condition that gets reclassified. A save directory we
/// genuinely cannot read is still a real error, and must keep warning loudly.
#[tokio::test]
#[cfg(unix)]
async fn an_unreadable_save_directory_is_still_an_io_error() {
    use std::os::unix::fs::PermissionsExt;

    let dir = empty_save_dir!("denied");
    fs::write(dir.join("Level.sav"), b"stub").expect("stub save");
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o000))
        .expect("drop permissions");

    let result = client(Some(dir.clone())).player_names(SourceKey::Shared).await;

    let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o755));
    let _ = fs::remove_dir_all(&dir);

    // Running as root bypasses the permission bits entirely; there is nothing to
    // assert in that case.
    match result {
        Err(PalworldError::Io(e)) => {
            assert_eq!(e.kind(), io::ErrorKind::PermissionDenied);
        },
        Err(PalworldError::NoWorld) => {
            panic!("a permission failure must not be reported as an empty world")
        },
        _ => {},
    }
}

/// Without a configured shared directory there is no world by definition, and
/// the warm must not touch the filesystem at all.
#[tokio::test]
async fn an_unconfigured_save_directory_is_no_world() {
    let err = client(None)
        .player_names(SourceKey::Shared)
        .await
        .expect_err("no configured save dir");

    assert!(matches!(err, PalworldError::NoWorld), "expected NoWorld, got {err:?}");
}
