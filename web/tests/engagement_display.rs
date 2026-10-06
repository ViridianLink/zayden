//! The text the engagement pages derive from their data.

use twilight_model::channel::ChannelType;
use web::auth::{ChannelInfo, RoleInfo};
use web::engagement::{
    CooldownView,
    GATE_KINDS,
    TEXT_KINDS,
    channel_label,
    custom_emoji_id,
    emoji_image_url,
    message_link,
    role_label,
    unconfigured_channels,
};
use web::guild::dto::Tier;

fn channel(id: &str, name: &str) -> ChannelInfo {
    ChannelInfo {
        id: id.to_owned(),
        name: name.to_owned(),
        kind: ChannelType::GuildText,
        tags: Vec::new(),
    }
}

fn role(id: &str, name: &str) -> RoleInfo {
    RoleInfo { id: id.to_owned(), name: name.to_owned(), color: 0 }
}

const fn cooldowns(next_tier: Option<Tier>) -> CooldownView {
    CooldownView {
        user_secs: 15,
        guild_secs: 3,
        floor_user_secs: 15,
        floor_guild_secs: 3,
        tier: Tier::Free,
        next_tier,
        next_floor_user_secs: 3,
        next_floor_guild_secs: 1,
    }
}

#[test]
fn custom_emoji_are_told_from_unicode_ones() {
    assert_eq!(custom_emoji_id("<:wave:123456789>"), Some("123456789"));
    assert_eq!(custom_emoji_id("<a:dance:42>"), Some("42"));
    assert_eq!(custom_emoji_id("\u{2705}"), None);
    assert_eq!(custom_emoji_id("<:wave:abc>"), None);
    assert_eq!(custom_emoji_id("<:wave:12"), None);
    assert_eq!(custom_emoji_id("<:wave:>"), None);
    assert_eq!(custom_emoji_id("<>"), None);
    assert_eq!(custom_emoji_id(":wave:12>"), None);
    assert_eq!(custom_emoji_id("plain"), None);
}

#[test]
fn custom_emoji_images_come_from_the_discord_cdn() {
    assert_eq!(
        emoji_image_url("42"),
        "https://cdn.discordapp.com/emojis/42.png?size=32"
    );
}

#[test]
fn message_links_open_the_message_in_discord() {
    assert_eq!(
        message_link("7", "20", "40"),
        "https://discord.com/channels/7/20/40"
    );
}

#[test]
fn channels_and_roles_are_named_or_marked_unknown() {
    let channels = [channel("20", "rules")];
    let roles = [role("30", "Member")];

    assert_eq!(channel_label(&channels, "20"), "#rules");
    assert_eq!(channel_label(&channels, "21"), "#unknown (21)");
    assert_eq!(role_label(&roles, "30"), "@Member");
    assert_eq!(role_label(&roles, "31"), "@unknown (31)");
}

#[test]
fn the_allow_picker_leaves_out_channels_already_allowed() {
    let channels =
        [channel("20", "rules"), channel("21", "chat"), channel("22", "bots")];

    let left = unconfigured_channels(&channels, &["21".to_owned()]);

    assert_eq!(left.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), ["20", "22"]);
    assert_eq!(unconfigured_channels(&channels, &[]).len(), 3);
}

#[test]
fn the_pickers_offer_the_channel_kinds_the_dashboard_offered() {
    assert_eq!(TEXT_KINDS, [ChannelType::GuildText, ChannelType::GuildAnnouncement]);
    assert_eq!(GATE_KINDS, [
        ChannelType::GuildText,
        ChannelType::GuildAnnouncement,
        ChannelType::GuildForum,
        ChannelType::GuildCategory,
    ]);
}

#[test]
fn the_cooldown_inputs_are_labelled_with_the_plans_floors() {
    let view = cooldowns(Some(Tier::Pro));

    assert_eq!(view.user_label(), "Per-member cooldown (seconds, min 15)");
    assert_eq!(view.guild_label(), "Server-wide cooldown (seconds, min 3)");
}

#[test]
fn only_a_plan_with_a_higher_one_pitches_an_upgrade() {
    assert_eq!(
        cooldowns(Some(Tier::Pro)).upgrade_pitch().as_deref(),
        Some("On Pro these floors drop to 3s and 1s.")
    );
    assert_eq!(cooldowns(None).upgrade_pitch(), None);
}
