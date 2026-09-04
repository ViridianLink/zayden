use ticket::faq::render::for_discord;
use ticket::wiki::WikiConfig;
use zayden_app::config::{FaqSettingsRow, SettingsRow};

/// A wiki pointed at a host that does not exist, which is all the render passes
/// need: they only ever ask it to build URLs.
#[expect(
    clippy::expect_used,
    reason = "a fixture that cannot be built is a broken test, not a runtime path"
)]
pub(crate) fn config() -> WikiConfig {
    let mut row = FaqSettingsRow::empty(1);
    row.enabled = true;
    row.wiki_url = Some(String::from("https://wiki.example.com"));

    WikiConfig::from_settings(&row)
        .expect("the fixture wiki url parses")
        .expect("the fixture row enables the wiki")
}

pub(crate) fn render(content: &str) -> String {
    for_discord(content, &config())
}
