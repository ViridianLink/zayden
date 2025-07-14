use serenity::all::{
    ButtonStyle, CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter, GenericChannelId,
    Mentionable, MessageId, ThreadId, UserId,
};

pub trait TemplateInfo {
    fn activity(&self) -> &str;

    fn timestamp(&self) -> i64;

    fn description(&self) -> &str;

    fn fireteam_size(&self) -> i16;

    fn fireteam(&self) -> impl Iterator<Item = UserId>;

    fn alternatives(&self) -> impl Iterator<Item = UserId>;

    fn schedule_channel(&self) -> Option<GenericChannelId>;

    fn alt_message(&self) -> Option<MessageId>;
}

pub trait Template {
    fn thread_embed<'a>(post: &'a impl TemplateInfo, owner_name: &str) -> CreateEmbed<'a>;

    fn message_embed<'a>(
        post: &'a impl TemplateInfo,
        owner_name: &str,
        thread: ThreadId,
    ) -> CreateEmbed<'a>;

    fn main_row<'a>() -> CreateActionRow<'a>;

    fn settings_row<'a>() -> CreateActionRow<'a> {
        CreateActionRow::buttons(vec![
            CreateButton::new("lfg_edit")
                .label("Edit")
                .style(ButtonStyle::Secondary),
            CreateButton::new("lfg_copy")
                .label("Copy")
                .style(ButtonStyle::Secondary),
            CreateButton::new("lfg_kick")
                .label("Kick")
                .style(ButtonStyle::Secondary),
            CreateButton::new("lfg_delete")
                .label("Delete")
                .style(ButtonStyle::Danger),
        ])
    }
}

pub struct DefaultTemplate;

impl Template for DefaultTemplate {
    fn thread_embed<'a>(post: &'a impl TemplateInfo, owner_name: &str) -> CreateEmbed<'a> {
        embed(post, owner_name, None)
    }

    fn message_embed<'a>(
        post: &'a impl TemplateInfo,
        owner_name: &str,
        thread: ThreadId,
    ) -> CreateEmbed<'a> {
        embed(post, owner_name, Some(thread))
    }

    fn main_row<'a>() -> CreateActionRow<'a> {
        CreateActionRow::buttons(vec![
            CreateButton::new("lfg_join")
                .emoji('➕')
                .style(ButtonStyle::Success),
            CreateButton::new("lfg_leave")
                .emoji('➖')
                .style(ButtonStyle::Danger),
            CreateButton::new("lfg_alternative")
                .emoji('❔')
                .style(ButtonStyle::Secondary),
            CreateButton::new("lfg_settings")
                .emoji('⚙')
                .style(ButtonStyle::Secondary),
        ])
    }
}

fn embed<'a>(
    post: &'a impl TemplateInfo,
    owner_name: &str,
    thread: Option<ThreadId>,
) -> CreateEmbed<'a> {
    let timestamp = post.timestamp();

    let fireteam = post
        .fireteam()
        .map(|id| id.mention().to_string())
        .collect::<Vec<_>>();

    let alternatives = post
        .alternatives()
        .map(|id| id.mention().to_string())
        .collect::<Vec<_>>();

    let fireteam_str = fireteam.join("\n");

    let mut embed = CreateEmbed::new()
        .title(format!("{} - <t:{}>", post.activity(), timestamp))
        .field("Activity", post.activity(), true)
        .field("Start Time", format!("<t:{timestamp}:R>"), true);

    if let Some(thread) = thread {
        embed = embed.field("Event Thread", thread.widen().mention().to_string(), true);
    }

    if !post.description().is_empty() {
        embed = embed.field("Description", post.description(), false)
    }

    embed = embed
        .field(
            format!("Joined: {}/{}", fireteam.len(), post.fireteam_size()),
            fireteam_str,
            false,
        )
        .footer(CreateEmbedFooter::new(format!("Posted by {owner_name}")));

    if !alternatives.is_empty() {
        embed = embed.field("Alternatives", alternatives.join("\n"), true);
    }

    embed
}
