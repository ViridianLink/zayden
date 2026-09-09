//! SRV-01: a single 46-field `GuildSettings` DTO plus a `SettingsBundle`
//! wrapper used to be fetched on every settings page load and handed to
//! all nine tabs - the Family tab used one field of it, the Patreon tab
//! used none, and `fetch_guild_settings` read all eleven settings
//! stores regardless of which tab was open. It is now a
//! section-independent `GuildDirectory` plus a tagged `SectionSettings`
//! enum, fetched and dispatched one section at a time.
//!
//! `src/dto/guild.rs`, `src/server/guild.rs` and
//! `src/ui/pages/guild_settings/` need a router, a live Postgres pool
//! and a Discord client to instantiate, so these assertions scan the
//! source text instead, matching the house style in
//! `web_state_split.rs`.

use std::fs;
use std::path::Path;

const CRATE_ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// `src/dto/guild.rs`, or empty text if it cannot be read; the vacuity
/// canary below catches that case.
fn dto_source() -> String {
    let path = Path::new(CRATE_ROOT).join("src/dto/guild.rs");
    fs::read_to_string(path).unwrap_or_default()
}

/// `src/server/guild.rs`, or empty text if it cannot be read; the
/// vacuity canary below catches that case.
fn server_source() -> String {
    let path = Path::new(CRATE_ROOT).join("src/server/guild.rs");
    fs::read_to_string(path).unwrap_or_default()
}

/// `src/ui/pages/guild_settings/mod.rs`, or empty text if it cannot be
/// read; the vacuity canary below catches that case.
fn shell_source() -> String {
    let path = Path::new(CRATE_ROOT).join("src/ui/pages/guild_settings/mod.rs");
    fs::read_to_string(path).unwrap_or_default()
}

/// A tab source file under `src/ui/pages/guild_settings/`, addressed by
/// a path fragment such as `"ai.rs"` or `"support/settings.rs"`. Empty
/// text if it cannot be read; the vacuity canary below catches that
/// case.
fn tab_source(name: &str) -> String {
    let path = Path::new(CRATE_ROOT).join("src/ui/pages/guild_settings").join(name);
    fs::read_to_string(path).unwrap_or_default()
}

/// Collapses every run of ASCII whitespace to a single space so
/// rustfmt line-wrapping cannot break a multi-token match.
fn squeezed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

const TAB_FILES: [&str; 9] = [
    "ai.rs",
    "family.rs",
    "general.rs",
    "honeypot.rs",
    "lfg.rs",
    "music.rs",
    "temp_voice.rs",
    "support/mod.rs",
    "support/settings.rs",
];

const SECTION_TYPES: [&str; 8] = [
    "AiSection",
    "FamilySection",
    "GeneralSection",
    "HoneypotSection",
    "LfgSection",
    "MusicSection",
    "TempVoiceSection",
    "SupportSection",
];

const SIMPLE_TABS: [(&str, &str); 7] = [
    ("ai.rs", "AiSection"),
    ("family.rs", "FamilySection"),
    ("general.rs", "GeneralSection"),
    ("honeypot.rs", "HoneypotSection"),
    ("lfg.rs", "LfgSection"),
    ("music.rs", "MusicSection"),
    ("temp_voice.rs", "TempVoiceSection"),
];

const SUPPORT_TABS: [(&str, &str); 2] = [
    ("support/mod.rs", "SupportSection"),
    ("support/settings.rs", "SupportSection"),
];

const SECTION_HELPERS: [&str; 8] = [
    "general_section",
    "ai_section",
    "family_section",
    "honeypot_section",
    "lfg_section",
    "music_section",
    "temp_voice_section",
    "support_section",
];

/// One DTO serving every tab is exactly what SRV-01 removed; neither
/// `GuildSettings` nor `SettingsBundle` may reappear in the split.
#[test]
fn the_monolithic_settings_dto_is_gone() {
    let mut files: Vec<(String, String)> = vec![
        ("src/dto/guild.rs".to_owned(), dto_source()),
        ("src/server/guild.rs".to_owned(), server_source()),
        ("src/ui/pages/guild_settings/mod.rs".to_owned(), shell_source()),
    ];

    for tab in TAB_FILES {
        let path = format!("src/ui/pages/guild_settings/{tab}");
        files.push((path, tab_source(tab)));
    }

    for (path, source) in &files {
        // GuildSettingsPage is the page component's name, not the
        // retired DTO; strip it first so it cannot false-positive the
        // shell file, which legitimately keeps that name.
        let without_page = source.replace("GuildSettingsPage", "");
        let squeezed_source = squeezed(&without_page);

        assert!(
            !squeezed_source.contains("GuildSettings"),
            "{path} still names GuildSettings - one DTO serving every \
             tab is exactly what SRV-01 removed"
        );
        assert!(
            !squeezed_source.contains("SettingsBundle"),
            "{path} still names SettingsBundle - one DTO serving \
             every tab is exactly what SRV-01 removed"
        );
    }
}

/// A tab that can see another tab's projection has re-acquired the
/// coupling SRV-01 removed; each tab must declare only its own section
/// type.
#[test]
fn each_tab_declares_only_its_own_projection() {
    for (file, type_name) in SIMPLE_TABS.into_iter().chain(SUPPORT_TABS) {
        let squeezed_source = squeezed(&tab_source(file));
        let pattern = format!("settings: {type_name}");

        assert!(
            squeezed_source.contains(&pattern),
            "{file} does not declare `{pattern}` - without its own \
             projection type the tab cannot be statically limited to \
             the fields it actually renders"
        );
    }

    for (file, own_type) in SIMPLE_TABS {
        let squeezed_source = squeezed(&tab_source(file));

        for foreign_type in SECTION_TYPES {
            if foreign_type == own_type {
                continue;
            }

            assert!(
                !squeezed_source.contains(foreign_type),
                "{file} names {foreign_type} - a tab that can see \
                 another tab's projection has re-acquired the \
                 coupling SRV-01 removed"
            );
        }
    }
}

/// Without a helper per section, `get_section_settings` cannot dispatch
/// to just one and reverts to reading every store on every load.
#[test]
fn the_server_dispatches_to_one_section_at_a_time() {
    let squeezed_server = squeezed(&server_source());

    assert!(
        squeezed_server.contains("pub async fn get_section_settings("),
        "src/server/guild.rs is missing pub async fn \
         get_section_settings( - the per-tab dispatcher itself is gone"
    );
    assert!(
        squeezed_server.contains("pub async fn get_guild_directory("),
        "src/server/guild.rs is missing pub async fn \
         get_guild_directory( - the section-independent channel/role \
         fetch is gone"
    );

    for helper in SECTION_HELPERS {
        let pattern = format!("fn {helper}(");
        assert!(
            squeezed_server.contains(&pattern),
            "src/server/guild.rs is missing `{pattern}` - without a \
             helper per section the dispatcher reverts to reading \
             every store"
        );
    }
}

/// The channel and role lists are uncached Discord calls, so keying
/// them on the section too would cost two Discord API calls on every
/// tab click; the settings resource must be keyed on the section or
/// the tab would render stale data.
#[test]
fn the_directory_is_keyed_on_the_guild_alone() {
    let squeezed_shell = squeezed(&shell_source());

    assert!(
        squeezed_shell
            .contains("Resource::new_blocking(guild_id, get_guild_directory)"),
        "src/ui/pages/guild_settings/mod.rs does not key the directory \
         resource on guild_id alone - the channel and role lists are \
         the same on every tab, and refetching them per section costs \
         two uncached Discord calls on every tab click"
    );
    assert!(
        squeezed_shell.contains("active.get().slug()"),
        "src/ui/pages/guild_settings/mod.rs does not key the settings \
         resource on active.get().slug() - without the section in the \
         key, switching tabs would keep rendering the previous tab's \
         data"
    );
}

/// Selecting the panel by matching the variant the server returned
/// makes it impossible to render one tab with another tab's data;
/// selecting it by slug string does not.
#[test]
fn the_rendered_tab_comes_from_the_returned_variant() {
    let squeezed_shell = squeezed(&shell_source());

    for variant in ["Support(", "Patreon(", "General("] {
        let pattern = format!("SectionSettings::{variant}");
        assert!(
            squeezed_shell.contains(&pattern),
            "src/ui/pages/guild_settings/mod.rs does not match \
             `{pattern}` - the panel must be selected from the \
             SectionSettings variant the server returned"
        );
    }

    assert!(
        !squeezed_shell.contains("Some(\"support\") =>"),
        "src/ui/pages/guild_settings/mod.rs matches on \
         Some(\"support\") => - selecting the panel by slug string \
         instead of the returned variant makes it possible to render \
         one tab with another tab's data"
    );
    assert!(
        !squeezed_shell.contains("Some(\"music\") =>"),
        "src/ui/pages/guild_settings/mod.rs matches on \
         Some(\"music\") => - selecting the panel by slug string \
         instead of the returned variant makes it possible to render \
         one tab with another tab's data"
    );
}

/// SEC-01 rides along with this rework: the raw Wiki.js key must stay
/// server-side, and the projection split must not have reintroduced it
/// into a DTO.
#[test]
fn the_wiki_api_key_never_enters_a_dto() {
    let squeezed_dto = squeezed(&dto_source());

    assert!(
        squeezed_dto.contains("wiki_api_key_set: bool"),
        "src/dto/guild.rs is missing wiki_api_key_set: bool - the FAQ \
         section must still expose only whether a key is saved"
    );
    assert!(
        !squeezed_dto.contains("wiki_api_key: String"),
        "src/dto/guild.rs declares wiki_api_key: String - the raw \
         Wiki.js key must never leave the server, which is the SEC-01 \
         invariant riding along with this projection rework"
    );
}

/// Every assertion above passes trivially against a scan that has
/// quietly stopped finding anything.
#[test]
fn the_scan_still_reaches_every_file() {
    let dto = dto_source();
    let server = server_source();
    let shell = shell_source();

    assert!(
        !dto.is_empty(),
        "src/dto/guild.rs came back empty - the scan is not reaching \
         dto/guild.rs"
    );
    assert!(
        !server.is_empty(),
        "src/server/guild.rs came back empty - the scan is not \
         reaching server/guild.rs"
    );
    assert!(
        !shell.is_empty(),
        "src/ui/pages/guild_settings/mod.rs came back empty - the \
         scan is not reaching guild_settings/mod.rs"
    );

    for tab in TAB_FILES {
        let source = tab_source(tab);
        assert!(
            !source.is_empty(),
            "src/ui/pages/guild_settings/{tab} came back empty - the \
             scan is not reaching that tab"
        );
    }

    assert!(
        dto.contains("enum SectionSettings"),
        "src/dto/guild.rs did not contain enum SectionSettings - the \
         file read back looks unrecognisable"
    );
    assert!(
        server.contains("fn get_section_settings"),
        "src/server/guild.rs did not contain fn get_section_settings \
         - the file read back looks unrecognisable"
    );
    assert!(
        shell.contains("GuildSettingsPage"),
        "src/ui/pages/guild_settings/mod.rs did not contain \
         GuildSettingsPage - the file read back looks unrecognisable"
    );
}
