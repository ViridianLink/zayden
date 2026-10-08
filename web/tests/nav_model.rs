use web::nav::{
    Dest,
    GENERAL,
    GROUPS,
    MODULES,
    ModuleNav,
    for_module,
    legacy_href,
    path_segment,
    section,
    settings_entry,
    settings_href,
};

#[test]
fn patreon_redirects_at_its_feature_page() {
    assert_eq!(
        settings_href("428610928000876544", "patreon").as_deref(),
        Some("/guild/428610928000876544/patreon")
    );
}

#[test]
fn youtube_redirects_at_its_feature_page() {
    assert_eq!(
        settings_href(428_610_928_000_876_544_i64, "youtube").as_deref(),
        Some("/guild/428610928000876544/youtube")
    );
}

#[test]
fn every_section_resolves_to_its_feature_page() {
    for (slug, path) in [
        ("general", "settings"),
        ("ai", "ai"),
        ("family", "family"),
        ("honeypot", "honeypot"),
        ("lfg", "lfg"),
        ("music", "music"),
        ("patreon", "patreon"),
        ("support", "support"),
        ("temp-voice", "temp-voice"),
        ("youtube", "youtube"),
    ] {
        assert_eq!(
            settings_href("1", slug).as_deref(),
            Some(format!("/guild/1/{path}").as_str())
        );
        assert_eq!(settings_entry(path).and_then(ModuleNav::slug), Some(slug));
    }
    assert_eq!(settings_entry("greetings"), None);
    assert_eq!(settings_entry("general"), None);
}

#[test]
fn a_legacy_slug_leads_to_the_page_that_now_holds_it() {
    for (slug, href) in [
        ("general", "/guild/7/settings"),
        ("ai", "/guild/7/ai"),
        ("support", "/guild/7/support"),
        ("temp-voice", "/guild/7/temp-voice"),
        ("greetings", "/guild/7/greetings"),
        ("levels", "/guild/7/levels"),
        ("reaction-roles", "/guild/7/reaction-roles"),
        ("bogus", "/guild/7/settings"),
        ("", "/guild/7/settings"),
    ] {
        assert_eq!(legacy_href(7, slug), href, "{slug}");
    }
}

#[test]
fn hrefs_encode_the_guild_segment() {
    assert_eq!(path_segment("7"), "7");
    assert_eq!(path_segment("a b/c?d"), "a%20b%2Fc%3Fd");
    assert_eq!(GENERAL.href("a/b"), "/guild/a%2Fb/settings");
    assert_eq!(legacy_href("x?y", "ai"), "/guild/x%3Fy/ai");
}

#[test]
fn an_unknown_slug_does_not_invent_a_route() {
    assert_eq!(settings_href("1", "patron"), None);
    assert_eq!(settings_href("1", "greetings"), None);
}

#[test]
fn the_module_list_keeps_its_labels_links_and_order() {
    let expected = [
        ("Server settings", "/guild/7/settings", None),
        ("Greetings", "/guild/7/greetings", Some("greetings")),
        ("Levels", "/guild/7/levels", None),
        ("Reaction roles", "/guild/7/reaction-roles", None),
        ("Family", "/guild/7/family", Some("family")),
        ("Support", "/guild/7/support", Some("ticket")),
        ("Honeypot", "/guild/7/honeypot", Some("honeypot")),
        ("Music", "/guild/7/music", Some("music")),
        ("Temp voice", "/guild/7/temp-voice", None),
        ("LFG", "/guild/7/lfg", None),
        ("AI Chat", "/guild/7/ai", Some("ai")),
        ("Patreon", "/guild/7/patreon", Some("patreon")),
        ("YouTube", "/guild/7/youtube", Some("youtube")),
    ];

    let actual: Vec<_> = MODULES
        .iter()
        .map(|module| (module.label, module.href(7), module.module_id))
        .collect();
    let expected: Vec<_> = expected
        .iter()
        .map(|(label, href, id)| (*label, (*href).to_owned(), *id))
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn the_groups_hold_every_entry_after_server_settings_in_order() {
    let groups: Vec<_> = GROUPS
        .iter()
        .map(|group| {
            let labels: Vec<_> =
                group.entries.iter().map(|entry| entry.label).collect();
            (group.label, labels)
        })
        .collect();

    assert_eq!(groups, [
        ("Community", vec!["Greetings", "Levels", "Reaction roles", "Family"]),
        ("Support & safety", vec!["Support", "Honeypot"]),
        ("Voice & games", vec!["Music", "Temp voice", "LFG"]),
        ("Integrations", vec!["AI Chat", "Patreon", "YouTube"]),
    ]);

    let (first, rest) = MODULES.split_first().unwrap();
    assert_eq!(first, &GENERAL);
    let grouped: Vec<_> = GROUPS.iter().flat_map(|group| group.entries).collect();
    assert_eq!(grouped, rest.iter().collect::<Vec<_>>());
}

#[test]
fn every_entry_keeps_its_lead() {
    let expected = [
        (
            "Server settings",
            "Server-wide channels and roles the rest of Zayden points at.",
        ),
        ("Greetings", ""),
        ("Levels", ""),
        ("Reaction roles", ""),
        ("Family", "Limits for the family and relationship commands."),
        (
            "Support",
            "Tickets, FAQ and suggestions - where they live and who gets pinged.",
        ),
        ("Honeypot", "The spam trap: a bait channel that bans whoever posts in it."),
        ("Music", "Playback permissions and now-playing announcements."),
        (
            "Temp voice",
            "On-demand voice channels created from a join-to-create channel.",
        ),
        ("LFG", "Where looking-for-group posts go and who they ping."),
        ("AI Chat", "Whether Zayden answers when mentioned, and where."),
        (
            "Patreon",
            "Connect a Patreon campaign and choose where its posts are announced.",
        ),
        (
            "YouTube",
            "Connect a YouTube channel and choose where its uploads are announced.",
        ),
    ];

    let actual: Vec<_> =
        MODULES.iter().map(|module| (module.label, module.lead())).collect();
    assert_eq!(actual, expected);
    for module in MODULES {
        if let Dest::Page(_) = module.dest {
            assert_eq!(module.slug(), None, "{}", module.label);
        }
    }
    assert_eq!(GENERAL.lead(), expected[0].1);
}

#[test]
fn an_unknown_section_falls_back_to_general() {
    assert_eq!(section("music").label, "Music");
    assert_eq!(section("nope"), &GENERAL);
    assert_eq!(section("levels"), &GENERAL);
}

#[test]
fn configure_links_follow_the_module_id() {
    let configurable: Vec<_> = [
        "ai",
        "family",
        "greetings",
        "honeypot",
        "music",
        "patreon",
        "ticket",
        "youtube",
    ]
    .into_iter()
    .map(|id| for_module(id).map(|module| module.label))
    .collect();

    assert_eq!(configurable, [
        Some("AI Chat"),
        Some("Family"),
        Some("Greetings"),
        Some("Honeypot"),
        Some("Music"),
        Some("Patreon"),
        Some("Support"),
        Some("YouTube")
    ]);
    assert_eq!(for_module("palworld"), None);
    assert_eq!(for_module("levels"), None);
}
