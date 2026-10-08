use web::document::PAGE_TITLES;
use web::nav::{Dest, MODULES, section};
use web::settings::title;

#[test]
fn section_titles_name_the_page() {
    for (slug, expected) in [
        ("general", "Server settings - Zayden Dashboard"),
        ("ai", "AI Chat - Zayden Dashboard"),
        ("family", "Family - Zayden Dashboard"),
        ("honeypot", "Honeypot - Zayden Dashboard"),
        ("lfg", "LFG - Zayden Dashboard"),
        ("music", "Music - Zayden Dashboard"),
        ("patreon", "Patreon - Zayden Dashboard"),
        ("support", "Support - Zayden Dashboard"),
        ("temp-voice", "Temp voice - Zayden Dashboard"),
        ("youtube", "YouTube - Zayden Dashboard"),
    ] {
        assert_eq!(title(section(slug)), expected, "{slug}");
    }
}

#[test]
fn missing_and_unknown_sections_title_as_server_settings() {
    for slug in ["", "bogus", "greetings", "levels", "reaction-roles", "General"] {
        assert_eq!(
            title(section(slug)),
            "Server settings - Zayden Dashboard",
            "{slug}"
        );
    }
}

/// The title table and the navigation label agree for every settings page.
#[test]
fn every_settings_page_has_its_navigation_label_as_title() {
    for entry in MODULES {
        let Dest::Section { path, .. } = entry.dest else { continue };
        let pattern = format!("/guild/{{guild_id}}/{path}");
        let row = PAGE_TITLES.iter().find(|(route, _)| *route == pattern);

        assert_eq!(
            row.map(|(_, t)| (*t).to_owned()),
            Some(title(entry)),
            "{pattern}"
        );
    }
}

#[test]
fn every_support_sub_page_has_a_title() {
    for (pattern, expected) in [
        ("/guild/{guild_id}/support/suggestions", "Suggestions - Support"),
        ("/guild/{guild_id}/support/faq", "FAQ articles - Support"),
        ("/guild/{guild_id}/support/faq/new", "New FAQ article - Support"),
        ("/guild/{guild_id}/support/faq/{article_id}", "Edit FAQ article - Support"),
        ("/guild/{guild_id}/support/wiki", "Wiki - Support"),
        ("/guild/{guild_id}/support/roles", "Roles and links - Support"),
    ] {
        let row = PAGE_TITLES.iter().find(|(route, _)| *route == pattern);
        assert_eq!(
            row.map(|(_, t)| *t),
            Some(format!("{expected} - Zayden Dashboard").as_str()),
            "{pattern}"
        );
    }
}
