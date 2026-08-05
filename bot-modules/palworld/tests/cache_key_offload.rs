//! Deriving a save cache key must not stat the disk on the reactor.
//!
//! The save *parsing* in `player_record` and `storage_pals` is correctly handed
//! to `spawn_blocking`, but the cache key those hops are keyed by is an mtime -
//! so the key derivation runs **before** the offload, on every `/pal` lookup,
//! including the ones the cache answers without parsing anything. These sit on
//! user-facing command paths rather than an unattended cron, and the shared
//! world is a mirror of a game server, so a `stat` is not reliably fast.
//!
//! Both guards assert the **first poll** is `Pending`. A future that stats
//! inline is `Ready` from that poll (or reaches its own offload having already
//! burned the reactor thread); one that hands the syscall to the blocking pool
//! cannot be, because the pool has not run it yet. Determinism comes from
//! capping the pool at one thread and occupying it first - so "has not run it"
//! is a guarantee rather than a bet on the syscall outlasting a single poll.

use std::future::Future;
use std::path::PathBuf;
use std::sync::mpsc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use palworld::client::{PalworldClient, SourceKey};
use palworld::save::dps;

/// A 32-hex-digit Palworld `PlayerUId`. Anything shorter fails to resolve to a
/// filename and short-circuits ahead of the stat this file is about.
const UID: &str = "00000000000000000000000000000001";

/// A path under the temp dir, unique per test and per process run.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join(format!("palworld-cache-key-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn client(uploads_dir: PathBuf) -> PalworldClient {
    PalworldClient::new(
        reqwest::Client::new(),
        None,
        None,
        None,
        None,
        uploads_dir,
        None,
    )
}

/// Poll `fut` exactly once on a runtime whose single blocking thread is already
/// taken, and report whether it came back `Pending`.
///
/// The gate releases on a timeout as well as on the sender drop, so a
/// regression **fails** on the assert instead of hanging on runtime shutdown.
fn first_poll_is_pending<F: Future>(fut: F) -> bool {
    let Ok(rt) = tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(1)
        .build()
    else {
        return false;
    };

    rt.block_on(async {
        let (release, hold) = mpsc::channel::<()>();
        let gate = tokio::task::spawn_blocking(move || {
            let _ = hold.recv_timeout(Duration::from_secs(5));
        });

        let mut fut = std::pin::pin!(fut);
        let pending = matches!(
            fut.as_mut().poll(&mut Context::from_waker(Waker::noop())),
            Poll::Pending
        );

        drop(release);
        // Only drive a future that is still running: re-polling one that already
        // returned `Ready` panics with "async fn resumed after completion",
        // which would mask the assertion the caller is about to make.
        if pending {
            let _ = fut.await;
        }
        let _ = gate.await;
        pending
    })
}

/// The `/pal` player lookup stats the save to build its cache key.
///
/// A *missing* save isolates that stat: the lookup returns `Ok(None)` the
/// moment the mtime comes back empty, without reaching the `spawn_blocking`
/// that parses the file. So the whole future used to resolve within one poll,
/// having done a synchronous `std::fs::metadata` on the reactor - which is the
/// defect. The stat is the same syscall on the cache-hit and parse paths; this
/// is only the case where nothing downstream can mask it.
#[test]
fn player_record_stats_the_save_off_the_reactor() {
    let uploads = scratch("player-record");
    std::fs::create_dir_all(uploads.join("1")).expect("uploads dir");

    let client = client(uploads.clone());
    let pending =
        first_poll_is_pending(client.player_record(SourceKey::User(1), UID));

    let _ = std::fs::remove_dir_all(&uploads);

    assert!(
        pending,
        "the player lookup resolved within a single poll, so it stat-ed the \
         save on the reactor thread; the cache key belongs on the blocking pool"
    );
}

/// The storage listing is worse than a single stat: a `read_dir` plus one stat
/// **per** `*_dps.sav` in the directory, all before the first parse is spawned.
#[test]
fn the_dps_listing_leaves_the_reactor_before_it_stats() {
    let dir = scratch("dps-listing");
    stage_dps(&dir, &["a", "b", "c"]).expect("stage saves");

    let pending = first_poll_is_pending(dps::list_files_with_mtime(&dir));

    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        pending,
        "the listing resolved within a single poll, so the read_dir and its \
         per-file stats ran on the reactor thread"
    );
}

/// Folding the stat into the listing must not change what the listing returns:
/// the same `*_dps.sav` set as before, each with a usable cache key.
#[tokio::test]
async fn the_listing_still_returns_only_storage_saves_with_their_mtimes() {
    let dir = scratch("dps-contents");
    stage_dps(&dir, &["a", "b"]).expect("stage saves");
    std::fs::write(dir.join("Players").join("plain.sav"), b"player")
        .expect("stage a non-storage save");

    let mut entries = dps::list_files_with_mtime(&dir).await;
    entries.sort();

    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(entries.len(), 2, "only the _dps.sav files: {entries:?}");
    for (path, mtime) in &entries {
        assert!(
            path.to_string_lossy().ends_with("_dps.sav"),
            "unexpected entry {path:?}"
        );
        assert_ne!(*mtime, 0, "{path:?} has no usable cache key");
    }
}

/// One `Players/<stem>_dps.sav` per stem, under a fresh save directory.
fn stage_dps(dir: &std::path::Path, stems: &[&str]) -> std::io::Result<()> {
    std::fs::create_dir_all(dir.join("Players"))?;
    for stem in stems {
        std::fs::write(dir.join("Players").join(format!("{stem}_dps.sav")), b"dps")?;
    }
    Ok(())
}
