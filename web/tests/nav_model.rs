use web::nav::{Dest, GENERAL, MODULES, for_module, section, settings_href};

#[test]
fn patreon_redirects_at_the_settings_section_route() {
    assert_eq!(
        settings_href("428610928000876544", "patreon").as_deref(),
        Some("/guild/428610928000876544/settings/patreon")
    );
}

#[test]
fn youtube_redirects_at_the_settings_section_route() {
    assert_eq!(
        settings_href(428_610_928_000_876_544_i64, "youtube").as_deref(),
        Some("/guild/428610928000876544/settings/youtube")
    );
}

#[test]
fn every_section_resolves_under_the_same_prefix() {
    for slug in [
        "general",
        "ai",
        "family",
        "honeypot",
        "lfg",
        "music",
        "patreon",
        "support",
        "temp-voice",
        "youtube",
    ] {
        assert_eq!(
            settings_href("1", slug).as_deref(),
            Some(format!("/guild/1/settings/{slug}").as_str())
        );
    }
}

#[test]
fn an_unknown_slug_does_not_invent_a_route() {
    assert_eq!(settings_href("1", "patron"), None);
    assert_eq!(settings_href("1", "greetings"), None);
}

#[test]
fn the_module_list_keeps_its_labels_links_and_order() {
    let expected = [
        ("General", "/guild/7/settings/general", None),
        ("AI Chat", "/guild/7/settings/ai", Some("ai")),
        ("Family", "/guild/7/settings/family", Some("family")),
        ("Greetings", "/guild/7/greetings", Some("greetings")),
        ("Honeypot", "/guild/7/settings/honeypot", Some("honeypot")),
        ("Levels", "/guild/7/levels", None),
        ("LFG", "/guild/7/settings/lfg", None),
        ("Music", "/guild/7/settings/music", Some("music")),
        ("Patreon", "/guild/7/settings/patreon", Some("patreon")),
        ("Reaction Roles", "/guild/7/reaction-roles", None),
        ("Support", "/guild/7/settings/support", Some("ticket")),
        ("Temp Voice", "/guild/7/settings/temp-voice", None),
        ("YouTube", "/guild/7/settings/youtube", Some("youtube")),
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
fn every_entry_keeps_its_lead() {
    let expected = [
        ("General", "Server-wide channels and roles the rest of Zayden points at."),
        ("AI Chat", "Whether Zayden answers when mentioned, and where."),
        ("Family", "Limits for the family and relationship commands."),
        ("Greetings", ""),
        ("Honeypot", "The spam trap: a bait channel that bans whoever posts in it."),
        ("Levels", ""),
        ("LFG", "Where looking-for-group posts go and who they ping."),
        ("Music", "Playback permissions and now-playing announcements."),
        (
            "Patreon",
            "Connect a Patreon campaign and choose where its posts are announced.",
        ),
        ("Reaction Roles", ""),
        (
            "Support",
            "Tickets, FAQ and suggestions - where they live and who gets pinged.",
        ),
        (
            "Temp Voice",
            "On-demand voice channels created from a join-to-create channel.",
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
