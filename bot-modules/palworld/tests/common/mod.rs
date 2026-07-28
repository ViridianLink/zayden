//! Paths shared by the save-reading test binaries.
//!
//! Every real-save assertion resolves its world through here, so the fixture can
//! be moved or replaced in one place.

use std::path::PathBuf;

/// A committed copy of a fully-progressed 8-player shared world.
///
/// This used to point at a working save at the workspace root, which meant the
/// assertions self-skipped the moment that directory was cleaned up or the
/// server rewrote it mid-read. It is now a fixture: always present, byte-stable,
/// and therefore never a reason to skip.
#[must_use]
pub fn progressed_world() -> PathBuf {
    fixture("progressed-world")
}

/// A committed copy of a 3-player shared world from the current game build - the
/// one that moved expanded Pal storage into `Players/<uid>_dps.sav`.
///
/// [`progressed_world`] is richer but Feybreak-era; anything that depends on the
/// format the server writes today belongs here.
#[must_use]
pub fn storage_world() -> PathBuf {
    fixture("storage-world")
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}
