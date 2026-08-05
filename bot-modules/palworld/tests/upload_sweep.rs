//! The expired-upload sweep must not do its filesystem work on the reactor.
//!
//! `PalworldUploadSweepCron` fires every minute and deletes the on-disk payload
//! of every expired upload. Each payload is a whole upload directory - up to the
//! tier's size cap - so removal is a recursive walk, not a single unlink. The
//! cron driver (`bot/src/cron.rs`) polls every due job's future together on one
//! task via `join_all`, so a sweep that unlinks inline stalls the runtime and
//! every other job in the same tick along with it.

use std::path::PathBuf;
use std::sync::mpsc;
use std::task::{Context, Waker};
use std::time::Duration;

use palworld::cron::remove_expired_uploads;

/// Stage one upload directory per id under a fresh temp root, and return the
/// paths as the sweep receives them: the `file_path` column, which points at
/// `Level.sav` *inside* the uploader's directory rather than at the directory.
fn uploads(tag: &str, ids: &[u64]) -> std::io::Result<(PathBuf, Vec<String>)> {
    let root = std::env::temp_dir()
        .join(format!("palworld_sweep_{tag}_{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    std::fs::create_dir_all(&root)?;

    let mut paths = Vec::with_capacity(ids.len());
    for id in ids {
        let dir = root.join(id.to_string());
        std::fs::create_dir_all(dir.join("Players"))?;
        std::fs::write(dir.join("Level.sav"), b"level")?;
        std::fs::write(dir.join("Players").join("a.sav"), b"player")?;
        paths.push(dir.join("Level.sav").to_string_lossy().into_owned());
    }

    Ok((root, paths))
}

/// The regression guard: the sweep's **first poll** must return `Pending`.
///
/// A future that removes the directories inline returns `Ready` from that first
/// poll, having already burned the reactor thread for the whole recursive walk -
/// which is exactly the defect. One that hands the work to `spawn_blocking`
/// cannot be `Ready` yet, because the blocking pool has not run it.
///
/// The pool is capped at one thread and that thread is occupied before the
/// sweep starts, so "has not run it" is a guarantee rather than a race: without
/// the gate the test would be betting that the removal outlasts the first poll
/// of its own join handle.
#[test]
fn sweep_hands_the_reactor_back_before_it_unlinks() {
    let (root, paths) = uploads("reactor", &[1, 2]).expect("stage uploads");

    let rt = tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(1)
        .build()
        .expect("build runtime");

    rt.block_on(async {
        // Occupy the only blocking thread. The timeout is the escape hatch for
        // the failing case: it lets the runtime shut down after the assert
        // below panics, so a regression fails loudly instead of hanging.
        let (release, hold) = mpsc::channel::<()>();
        let gate = tokio::task::spawn_blocking(move || {
            let _ = hold.recv_timeout(Duration::from_secs(5));
        });

        let mut sweep = std::pin::pin!(remove_expired_uploads(paths));
        let poll = sweep.as_mut().poll(&mut Context::from_waker(Waker::noop()));

        assert!(
            poll.is_pending(),
            "the sweep completed within a single poll, so it unlinked on the \
             reactor thread; the removal belongs on the blocking pool"
        );

        drop(release);
        sweep.await;
        gate.await.expect("gate task");
    });

    assert!(!root.join("1").exists(), "upload 1 removed");
    assert!(!root.join("2").exists(), "upload 2 removed");
    std::fs::remove_dir_all(&root).ok();
}

/// One unremovable path must not cost the rest of the tick. The bad path is
/// first, so a loop that aborts on error leaves both real payloads on disk.
#[tokio::test]
async fn one_bad_path_does_not_abandon_the_remaining_payloads() {
    let (root, mut paths) = uploads("resilience", &[7, 8]).expect("stage uploads");
    let missing = root.join("gone").join("Level.sav").to_string_lossy().into_owned();
    paths.insert(0, missing);

    remove_expired_uploads(paths).await;

    assert!(!root.join("7").exists(), "upload 7 removed despite the bad path");
    assert!(!root.join("8").exists(), "upload 8 removed despite the bad path");
    std::fs::remove_dir_all(&root).ok();
}

/// The whole payload directory goes, not just the `Level.sav` the column names.
#[tokio::test]
async fn sweep_removes_the_whole_upload_directory() {
    let (root, paths) = uploads("payload", &[42]).expect("stage uploads");

    remove_expired_uploads(paths).await;

    assert!(
        !root.join("42").exists(),
        "the sibling Players/ saves go with the Level.sav, or the upload's \
         quota is never actually reclaimed"
    );
    std::fs::remove_dir_all(&root).ok();
}
