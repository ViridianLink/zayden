//! Source scans over the server code in `src`: the leaderboard asks for one
//! row past its page, the settings page loads one section at a time, the
//! shared state stays split by concern and the legal pages need no session.

use std::fs;
use std::path::Path;

const CRATE_ROOT: &str = env!("CARGO_MANIFEST_DIR");

const SECTION_TABS: [(&str, &str); 9] = [
    ("ai.rs", "AiSection"),
    ("family.rs", "FamilySection"),
    ("general.rs", "GeneralSection"),
    ("honeypot.rs", "HoneypotSection"),
    ("lfg.rs", "LfgSection"),
    ("music.rs", "MusicSection"),
    ("temp_voice.rs", "TempVoiceSection"),
    ("support/mod.rs", "SupportSection"),
    ("support/settings.rs", "SupportSection"),
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

/// The file's text, or empty text if it cannot be read; the canaries below
/// catch that case.
fn read(relative: &str) -> String {
    fs::read_to_string(Path::new(CRATE_ROOT).join(relative)).unwrap_or_default()
}

/// Collapses every run of whitespace to a single space so line wrapping
/// cannot break a multi-token match.
fn squeezed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn collect(dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else { return };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs")
            && let Ok(text) = fs::read_to_string(&path)
        {
            out.push(format!(
                "{}\u{0}{text}",
                path.strip_prefix(CRATE_ROOT).unwrap_or(&path).display()
            ));
        }
    }
}

#[test]
fn the_leaderboard_does_not_infer_the_next_page() {
    let page = read("src/engagement/pages/levels.rs");

    assert!(
        !page.contains("entries.len() =="),
        "the page must take has_next from the response, not guess it from the row count"
    );
    assert!(
        !page.contains("PAGE_SIZE"),
        "the page must not keep its own copy of the page size"
    );
    assert!(squeezed(&page).contains("page.has_next"));
}

#[test]
fn the_leaderboard_fetches_one_row_past_the_page() {
    let server = squeezed(&read("src/engagement/levels.rs"));

    assert!(server.contains("let limit = PAGE_SIZE + 1;"));
    assert!(server.contains("LIMIT $1 OFFSET $2"));
    assert!(server.contains("LIMIT $2 OFFSET $3"));
    assert!(
        !server.contains("LIMIT 10"),
        "a fixed LIMIT 10 cannot tell a full page from the last"
    );
}

#[test]
fn the_probe_row_never_reaches_the_page() {
    let server = squeezed(&read("src/engagement/levels.rs"));

    assert!(server.contains("(offset + 1..=offset + PAGE_SIZE).zip(&rows)"));
    assert!(server.contains("let has_next = rows.len() > entries.len();"));
}

#[test]
fn the_pager_escapes_an_empty_page() {
    let page = squeezed(&read("src/engagement/pages/levels.rs"));
    let view = squeezed(&read("src/engagement/leaderboard_view.rs"));

    assert!(page.contains("if board.shows_pager(page.has_next) {"));
    assert!(page.contains("class=\"pager\""));
    assert!(view.contains("self.page > 1 || has_next"));
    assert!(!page.contains("entries.is_empty() =>"));
}

#[test]
fn the_leaderboard_scan_still_reaches_both_files() {
    assert!(read("src/engagement/levels.rs").contains("get_leaderboard"));
    assert!(read("src/engagement/pages/levels.rs").contains("fn leaderboard"));
}

#[test]
fn the_monolithic_settings_dto_is_gone() {
    let mut files = vec![
        ("src/guild/dto/guild.rs".to_owned(), read("src/guild/dto/guild.rs")),
        ("src/guild/sections.rs".to_owned(), read("src/guild/sections.rs")),
        ("src/settings/mod.rs".to_owned(), read("src/settings/mod.rs")),
    ];
    for (tab, _) in SECTION_TABS {
        let path = format!("src/settings/{tab}");
        let text = read(&path);
        files.push((path, text));
    }

    for (path, source) in &files {
        let source = squeezed(source);
        assert!(!source.contains("GuildSettings"), "{path} names GuildSettings");
        assert!(!source.contains("SettingsBundle"), "{path} names SettingsBundle");
    }
}

#[test]
fn each_tab_declares_only_its_own_projection() {
    for (file, type_name) in SECTION_TABS {
        let source = squeezed(&read(&format!("src/settings/{file}")));
        let pattern = format!("settings: &{type_name}");

        assert!(source.contains(&pattern), "{file} does not declare `{pattern}`");
    }

    for (file, own) in SECTION_TABS {
        if file.starts_with("support/") {
            continue;
        }
        let source = squeezed(&read(&format!("src/settings/{file}")));

        for foreign in SECTION_TYPES {
            assert!(
                foreign == own || !source.contains(foreign),
                "{file} names {foreign}, another tab's projection"
            );
        }
    }
}

#[test]
fn the_server_dispatches_to_one_section_at_a_time() {
    let sections = squeezed(&read("src/guild/sections.rs"));

    assert!(sections.contains("pub async fn get_section_settings("));
    assert!(
        squeezed(&read("src/guild/directory.rs"))
            .contains("pub async fn get_guild_directory(")
    );

    for helper in SECTION_HELPERS {
        let pattern = format!("fn {helper}(");
        assert!(sections.contains(&pattern), "missing `{pattern}`");
    }
}

#[test]
fn the_directory_is_loaded_for_the_guild_and_the_settings_for_the_section() {
    let shell = squeezed(&read("src/settings/mod.rs"));

    assert_eq!(1, shell.matches("get_guild_directory(cx, guild_id)").count());
    assert_eq!(1, shell.matches("get_section_settings(cx, guild_id, slug)").count());
}

#[test]
fn the_rendered_tab_comes_from_the_returned_variant() {
    let shell = squeezed(&read("src/settings/mod.rs"));

    for variant in ["Support(", "Patreon(", "General("] {
        let pattern = format!("SectionSettings::{variant}");
        assert!(shell.contains(&pattern), "the panel does not match `{pattern}`");
    }
    assert!(!shell.contains("Some(\"support\") =>"));
    assert!(!shell.contains("Some(\"music\") =>"));
}

#[test]
fn the_wiki_api_key_never_enters_a_dto() {
    let dto = squeezed(&read("src/guild/dto/guild.rs"));

    assert!(dto.contains("wiki_api_key_set: bool"));
    assert!(!dto.contains("wiki_api_key: String"));
}

#[test]
fn the_settings_scan_still_reaches_every_file() {
    assert!(read("src/guild/dto/guild.rs").contains("enum SectionSettings"));
    assert!(read("src/guild/sections.rs").contains("fn get_section_settings"));
    assert!(read("src/settings/mod.rs").contains("fn settings_page"));

    for (tab, _) in SECTION_TABS {
        assert!(
            !read(&format!("src/settings/{tab}")).is_empty(),
            "{tab} read back empty"
        );
    }
}

#[test]
fn the_shared_state_stays_grouped_by_concern() {
    let state = squeezed(&read("src/state.rs"));

    for field in [
        "pub app: Arc<ZaydenAppState>,",
        "pub sessions: SessionCache,",
        "pub oauth: OAuthState,",
        "pub discord: DiscordState,",
        "pub integrations: IntegrationsState,",
        "pub urls: SiteUrls,",
    ] {
        assert!(state.contains(field), "state.rs lost `{field}`");
    }
}

#[test]
fn the_whole_state_is_fetched_only_by_the_context_accessors() {
    let mut files = Vec::new();
    collect(&Path::new(CRATE_ROOT).join("src"), &mut files);

    let readers: Vec<_> = files
        .iter()
        .filter_map(|entry| entry.split_once('\u{0}'))
        .filter(|(_, text)| text.contains("app_context::<WebState>"))
        .map(|(path, _)| path.to_owned())
        .collect();

    assert!(files.len() > 40, "only {} sources scanned", files.len());
    assert_eq!(vec!["src/auth/context.rs".to_owned()], readers);
}

#[test]
fn the_route_modules_only_name_what_they_use() {
    let kofi = read("src/providers/kofi.rs");
    let login = read("src/auth/login.rs");
    let patreon = read("src/providers/patreon.rs");
    let youtube = read("src/providers/youtube.rs");

    for group in [".oauth", ".discord", ".urls"] {
        assert!(!kofi.contains(group), "the Ko-fi route reaches into {group}");
    }
    for group in [".integrations", ".discord"] {
        assert!(!login.contains(group), "the sign-in routes reach into {group}");
    }
    for source in [&patreon, &youtube] {
        assert!(!source.contains(".oauth"), "a provider route reaches into .oauth");
    }

    let middleware = read("src/auth/middleware.rs");
    for group in ["web_state", ".oauth", ".discord", ".integrations", ".urls"] {
        assert!(
            !middleware.contains(group),
            "the auth middleware reaches into {group}"
        );
    }
    assert!(middleware.contains("session_for_token"));

    assert!(kofi.contains(".integrations"));
    assert!(login.contains(".oauth"));
    assert!(patreon.contains(".integrations") && youtube.contains(".integrations"));
}

#[test]
fn the_legal_pages_need_no_session_or_database() {
    for file in [
        "src/public/privacy.rs",
        "src/public/terms.rs",
        "src/public/legal.rs",
        "src/components/legal.rs",
    ] {
        let source = read(file);

        assert!(!source.is_empty(), "{file} read back empty");
        for needle in [
            "crate::auth",
            "crate::guild",
            "crate::shell",
            "web_state",
            "db_pool",
            "cookies(",
        ] {
            assert!(!source.contains(needle), "{file} reaches for `{needle}`");
        }
    }
}
