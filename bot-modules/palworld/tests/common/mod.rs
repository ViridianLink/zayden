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

/// A small (113 KB) single-player world - the cheapest fixture that still
/// exercises the full container and GVAS paths.
#[must_use]
pub fn steam_world1() -> PathBuf {
    fixture("steam-world1")
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

/// Reading a world costs ~3.2s, almost all of it in `read_gvas` parsing the
/// 2.7 MB `Level.sav`; cloning the parsed result costs ~180ms. The binaries
/// below assert against the same few worlds from a dozen tests each, so each
/// world is decoded once per binary and handed out by reference.
///
/// The accessors return `Option` rather than panicking because the workspace
/// denies `expect_used` outside `#[test]` fns — callers unwrap inside the test
/// body, the same reason [`level_bytes`] is a macro.
mod cache {
    use std::path::Path;
    use std::sync::LazyLock;

    use gvas::GvasFile;
    use palworld::model::{PlayerName, WorldRoster};
    use palworld::save::decompress::decompress;
    use palworld::save::edit::{SaveRoster, read_roster};
    use palworld::save::gvas::read_gvas;
    use palworld::save::{load_player_names, load_world};

    pub(super) fn level_bytes(dir: &Path) -> Option<Vec<u8>> {
        std::fs::read(dir.join("Level.sav")).ok()
    }

    fn gvas_of(dir: &Path) -> Option<GvasFile> {
        read_gvas(&decompress(&level_bytes(dir)?).ok()?).ok()
    }

    pub(super) static PROGRESSED_LEVEL: LazyLock<Option<Vec<u8>>> =
        LazyLock::new(|| level_bytes(&super::progressed_world()));
    pub(super) static STORAGE_LEVEL: LazyLock<Option<Vec<u8>>> =
        LazyLock::new(|| level_bytes(&super::storage_world()));
    pub(super) static STEAM_LEVEL: LazyLock<Option<Vec<u8>>> =
        LazyLock::new(|| level_bytes(&super::steam_world1()));

    pub(super) static PROGRESSED_GVAS: LazyLock<Option<GvasFile>> =
        LazyLock::new(|| gvas_of(&super::progressed_world()));
    pub(super) static STORAGE_GVAS: LazyLock<Option<GvasFile>> =
        LazyLock::new(|| gvas_of(&super::storage_world()));
    pub(super) static STEAM_GVAS: LazyLock<Option<GvasFile>> =
        LazyLock::new(|| gvas_of(&super::steam_world1()));

    pub(super) static PROGRESSED_ROSTER: LazyLock<Option<SaveRoster>> =
        LazyLock::new(|| read_roster(PROGRESSED_LEVEL.as_ref()?, 0).ok());

    pub(super) static PROGRESSED_NAMES: LazyLock<Option<Vec<PlayerName>>> =
        LazyLock::new(|| load_player_names(&super::progressed_world()).ok());
    pub(super) static STORAGE_NAMES: LazyLock<Option<Vec<PlayerName>>> =
        LazyLock::new(|| load_player_names(&super::storage_world()).ok());

    pub(super) static PROGRESSED_WORLD_ROSTER: LazyLock<Option<WorldRoster>> =
        LazyLock::new(|| load_world(&super::progressed_world()).ok());
    pub(super) static STORAGE_WORLD_ROSTER: LazyLock<Option<WorldRoster>> =
        LazyLock::new(|| load_world(&super::storage_world()).ok());
}

/// Raw `Level.sav` bytes, read once per test binary.
#[must_use]
pub fn progressed_level() -> Option<&'static [u8]> {
    cache::PROGRESSED_LEVEL.as_deref()
}

/// Raw `Level.sav` bytes, read once per test binary.
#[must_use]
pub fn storage_level() -> Option<&'static [u8]> {
    cache::STORAGE_LEVEL.as_deref()
}

/// Raw `Level.sav` bytes, read once per test binary.
#[must_use]
pub fn steam_world1_level() -> Option<&'static [u8]> {
    cache::STEAM_LEVEL.as_deref()
}

/// The decoded `Level.sav`, parsed once per test binary. Clone it when a test
/// needs to mutate; that is ~16x cheaper than re-parsing.
#[must_use]
pub fn progressed_gvas() -> Option<&'static gvas::GvasFile> {
    cache::PROGRESSED_GVAS.as_ref()
}

/// The decoded `Level.sav`, parsed once per test binary.
#[must_use]
pub fn storage_gvas() -> Option<&'static gvas::GvasFile> {
    cache::STORAGE_GVAS.as_ref()
}

/// The decoded `Level.sav`, parsed once per test binary.
#[must_use]
pub fn steam_world1_gvas() -> Option<&'static gvas::GvasFile> {
    cache::STEAM_GVAS.as_ref()
}

/// `read_roster` at `level_modified = 0`, decoded once per test binary.
#[must_use]
pub fn progressed_roster() -> Option<&'static palworld::save::edit::SaveRoster> {
    cache::PROGRESSED_ROSTER.as_ref()
}

/// `load_player_names`, decoded once per test binary.
#[must_use]
pub fn progressed_names() -> Option<&'static [palworld::model::PlayerName]> {
    cache::PROGRESSED_NAMES.as_deref()
}

/// `load_player_names`, decoded once per test binary.
#[must_use]
pub fn storage_names() -> Option<&'static [palworld::model::PlayerName]> {
    cache::STORAGE_NAMES.as_deref()
}

/// `load_world`, decoded once per test binary.
#[must_use]
pub fn progressed_world_roster() -> Option<&'static palworld::model::WorldRoster> {
    cache::PROGRESSED_WORLD_ROSTER.as_ref()
}

/// `load_world`, decoded once per test binary.
#[must_use]
pub fn storage_world_roster() -> Option<&'static palworld::model::WorldRoster> {
    cache::STORAGE_WORLD_ROSTER.as_ref()
}
