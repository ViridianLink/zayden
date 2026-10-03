//! Engagement forms read their urlencoded pairs strictly: each expected field
//! exactly once and nothing else, with the guild named by the path.

use web::engagement::EngagementError;
use web::engagement::greetings::{
    AddGreetingImageForm,
    GreetingChannelForm,
    RemoveGreetingImageForm,
    SaveGreetingCooldownsForm,
    SaveGreetingMessagesForm,
};
use web::engagement::reaction_roles::{AddReactionRoleForm, RemoveReactionRoleForm};

fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
    items.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect()
}

#[test]
fn the_reaction_role_forms_read_the_dashboards_field_names() {
    let add = AddReactionRoleForm::from_pairs(pairs(&[
        ("emoji", "\u{2705}"),
        ("role_id", "30"),
        ("message_id", ""),
        ("channel_id", "20"),
        ("guild", "7"),
    ]))
    .unwrap();

    assert_eq!(add.guild, "7");
    assert_eq!(add.channel_id, "20");
    assert_eq!(add.message_id, "");
    assert_eq!(add.role_id, "30");
    assert_eq!(add.emoji, "\u{2705}");

    let remove = RemoveReactionRoleForm::from_pairs(pairs(&[
        ("guild", "7"),
        ("channel_id", "20"),
        ("message_id", "40"),
        ("emoji", "<:a:5>"),
    ]))
    .unwrap();

    assert_eq!(remove.message_id, "40");
    assert_eq!(remove.emoji, "<:a:5>");
}

#[test]
fn the_greeting_forms_read_the_dashboards_field_names() {
    let messages = SaveGreetingMessagesForm::from_pairs(pairs(&[
        ("guild", "7"),
        ("morning_message", " hi {user} "),
        ("night_message", ""),
    ]))
    .unwrap();
    assert_eq!(messages.morning_message, " hi {user} ");
    assert_eq!(messages.night_message, "");

    let cooldowns = SaveGreetingCooldownsForm::from_pairs(pairs(&[
        ("guild", "7"),
        ("user_cooldown", "15"),
        ("guild_cooldown", ""),
    ]))
    .unwrap();
    assert_eq!(cooldowns.user_cooldown, "15");
    assert_eq!(cooldowns.guild_cooldown, "");

    let channel = GreetingChannelForm::from_pairs(pairs(&[
        ("guild", "7"),
        ("channel_id", "9"),
    ]))
    .unwrap();
    assert_eq!(channel.channel_id, "9");

    let image = AddGreetingImageForm::from_pairs(pairs(&[
        ("guild", "7"),
        ("kind", "morning"),
        ("url", "https://example.com/a.gif"),
    ]))
    .unwrap();
    assert_eq!(image.kind, "morning");
    assert_eq!(image.url, "https://example.com/a.gif");

    let remove =
        RemoveGreetingImageForm::from_pairs(pairs(&[("id", "3"), ("guild", "7")]))
            .unwrap();
    assert_eq!(remove.id, "3");
}

#[test]
fn an_unknown_field_is_refused() {
    let err = GreetingChannelForm::from_pairs(pairs(&[
        ("guild", "7"),
        ("channel_id", "9"),
        ("extra", "x"),
    ]))
    .unwrap_err();

    assert_eq!(err, EngagementError::UnknownField("extra".to_owned()));
    assert!(err.is_invalid_form());
}

#[test]
fn a_repeated_field_is_refused() {
    let err = GreetingChannelForm::from_pairs(pairs(&[
        ("guild", "7"),
        ("channel_id", "9"),
        ("channel_id", "10"),
    ]))
    .unwrap_err();

    assert_eq!(err, EngagementError::DuplicateField("channel_id".to_owned()));
    assert!(err.is_invalid_form());
}

#[test]
fn a_missing_field_is_refused() {
    let err = AddReactionRoleForm::from_pairs(pairs(&[
        ("guild", "7"),
        ("channel_id", "20"),
        ("message_id", ""),
        ("role_id", "30"),
    ]))
    .unwrap_err();

    assert_eq!(err, EngagementError::MissingField("emoji"));
    assert!(err.is_invalid_form());
}

#[test]
fn a_form_for_another_guild_than_the_path_is_refused() {
    let form =
        RemoveGreetingImageForm::from_pairs(pairs(&[("guild", "7"), ("id", "3")]))
            .unwrap();

    assert_eq!(form.ensure_path_guild("7"), Ok(()));

    let err = form.ensure_path_guild("8").unwrap_err();
    assert_eq!(err, EngagementError::GuildMismatch);
    assert!(err.is_invalid_form());
}
