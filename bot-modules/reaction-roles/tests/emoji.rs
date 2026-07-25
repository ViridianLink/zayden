use reaction_roles::ParsedEmoji;
use serenity::all::ReactionType;

/// The mapping is looked up by `ReactionRole::row(pool, message_id, &emoji)`
/// where `emoji` is `reaction.emoji.to_string()` (see `src/reaction/mod.rs`).
/// So whatever the dashboard writes into `reaction_roles.emoji` must be
/// byte-identical to that rendering, or the reaction fires and matches
/// nothing — the mapping is silently inert.
///
/// This pins `ParsedEmoji::parse` against the gateway's own rendering for each
/// emoji shape an admin can type.
#[test]
fn stored_form_matches_gateway_rendering() {
    for input in [
        "\u{2705}",
        "\u{1F389}",
        "<:customemoji:600404340292059257>",
        "<a:spin:600404340292059258>",
    ] {
        let parsed = ParsedEmoji::parse(input).unwrap();
        let gateway = ReactionType::try_from(input).unwrap().to_string();

        assert_eq!(
            parsed.stored, gateway,
            "stored form for {input} must match the reaction handler's lookup key"
        );
    }
}

/// Admins paste emoji out of Discord with stray whitespace; the trimmed value
/// is what gets stored, so a padded input and a clean one address the same row.
#[test]
fn surrounding_whitespace_is_trimmed() {
    assert_eq!(ParsedEmoji::parse("  \u{2705} ").unwrap().stored, "\u{2705}");
    assert_eq!(
        ParsedEmoji::parse(" <:name:600404340292059257>\n").unwrap().stored,
        "<:name:600404340292059257>"
    );
}

/// The dashboard turns `custom_id`/`name` into a twilight `RequestReactionType`
/// to seed the reaction. A custom emoji must carry its id, a unicode one must
/// not — swapping the two makes the seeded reaction 400 at the API.
#[test]
fn custom_emoji_exposes_its_id_and_name() {
    let custom = ParsedEmoji::parse("<:customemoji:600404340292059257>").unwrap();
    assert_eq!(custom.custom_id, Some(600_404_340_292_059_257));
    assert_eq!(custom.name, "customemoji");

    let animated = ParsedEmoji::parse("<a:spin:600404340292059258>").unwrap();
    assert_eq!(animated.custom_id, Some(600_404_340_292_059_258));
    assert_eq!(animated.name, "spin");
    assert_eq!(animated.stored, "<a:spin:600404340292059258>");

    let unicode = ParsedEmoji::parse("\u{2705}").unwrap();
    assert_eq!(unicode.custom_id, None);
    assert_eq!(unicode.name, "\u{2705}");
}

#[test]
fn malformed_input_is_rejected() {
    assert!(ParsedEmoji::parse("").is_err());
    assert!(ParsedEmoji::parse("   ").is_err());
    assert!(ParsedEmoji::parse("<:missing_id:").is_err());
    assert!(ParsedEmoji::parse("<:notanumber:abc>").is_err());
}
