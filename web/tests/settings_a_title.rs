use web::settings::title;

#[test]
fn section_titles_name_the_section() {
    for (slug, expected) in [
        ("general", "General settings - Zayden Dashboard"),
        ("ai", "AI Chat settings - Zayden Dashboard"),
        ("family", "Family settings - Zayden Dashboard"),
        ("honeypot", "Honeypot settings - Zayden Dashboard"),
        ("lfg", "LFG settings - Zayden Dashboard"),
        ("music", "Music settings - Zayden Dashboard"),
        ("patreon", "Patreon settings - Zayden Dashboard"),
        ("support", "Support settings - Zayden Dashboard"),
        ("temp-voice", "Temp Voice settings - Zayden Dashboard"),
        ("youtube", "YouTube settings - Zayden Dashboard"),
    ] {
        assert_eq!(title(slug), expected, "{slug}");
    }
}

#[test]
fn missing_and_unknown_sections_title_as_general() {
    for slug in ["", "bogus", "greetings", "levels", "reaction-roles", "General"] {
        assert_eq!(title(slug), "General settings - Zayden Dashboard", "{slug}");
    }
}
