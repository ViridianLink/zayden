use futures::{StreamExt, TryStreamExt};
use serenity::all::{
    ButtonStyle, CreateActionRow, CreateButton, CreateComponent, CreateEmbed, CreateEmbedAuthor,
    CreateEmbedFooter, CreateMessage, EditMessage, EmbedField, GuildChannel, Http, Message,
    Reaction, ReactionType,
};
use sqlx::{Database, Pool};

use crate::{Suggestions, SuggestionsGuildManager};

impl Suggestions {
    pub async fn reaction<Db: Database, Manager: SuggestionsGuildManager<Db>>(
        http: &Http,
        reaction: &Reaction,
        pool: &Pool<Db>,
    ) {
        let Some(guild_id) = reaction.guild_id else {
            return;
        };

        let Some(channel) = reaction.channel(&http).await.unwrap().guild() else {
            return;
        };

        let Some(row) = Manager::get(pool, guild_id).await.unwrap() else {
            return;
        };

        if channel.parent_id.is_none()
            || row.channel_id().is_none()
            || channel.parent_id != row.channel_id()
        {
            return;
        }

        let Some(review_channel_id) = row.review_channel_id() else {
            return;
        };

        let message = reaction.message(http).await.unwrap();

        let positive_reaction = ReactionType::from('👍');
        let negative_reaction = ReactionType::from('👎');

        let (pos_count, neg_count) = message.reactions.iter().fold((0, 0), |(pos, neg), r| {
            if r.reaction_type == positive_reaction {
                (r.count as i32, neg)
            } else if r.reaction_type == negative_reaction {
                (pos, r.count as i32)
            } else {
                (pos, neg)
            }
        });

        let mut messages = review_channel_id.widen().messages_iter(&http).boxed();

        if (pos_count - neg_count) >= 20 {
            while let Some(mut msg) = messages.try_next().await.unwrap() {
                if let Some(embed) = msg.embeds.first()
                    && embed.url.as_deref() == Some(message.link().as_str())
                {
                    let embed = create_embed(
                        &channel,
                        &message,
                        &msg.embeds[0].fields,
                        pos_count,
                        neg_count,
                    );

                    msg.edit(
                        http,
                        EditMessage::new()
                            .embed(embed)
                            .components(vec![create_components()]),
                    )
                    .await
                    .unwrap();

                    return;
                }
            }

            review_channel_id
                .widen()
                .send_message(
                    http,
                    CreateMessage::new()
                        .embed(create_embed(
                            &channel,
                            &message,
                            &Vec::new(),
                            pos_count,
                            neg_count,
                        ))
                        .components(vec![create_components()]),
                )
                .await
                .unwrap();
        } else if (neg_count - pos_count) <= 15 {
            while let Some(msg) = messages.try_next().await.unwrap() {
                if msg.embeds[0].url.as_deref() == Some(message.link().as_str()) {
                    msg.delete(http, Some("Positive delta fell below 15"))
                        .await
                        .unwrap();

                    return;
                }
            }
        }
    }
}

fn create_embed<'a>(
    channel: &'a GuildChannel,
    message: &'a Message,
    embed_fields: &[EmbedField],
    pos_count: i32,
    neg_count: i32,
) -> CreateEmbed<'a> {
    let mut embed = CreateEmbed::new()
        .title(&channel.base.name)
        .url(message.link())
        .description(&message.content)
        .author(CreateEmbedAuthor::new(&message.author.name))
        .footer(CreateEmbedFooter::new(format!(
            "👍 {pos_count} · 👎 {neg_count}",
        )));

    if let Some(team_response) = embed_fields.first() {
        embed = embed.field(
            team_response.name.clone(),
            team_response.value.clone(),
            team_response.inline,
        );
    }

    embed
}

fn create_components<'a>() -> CreateComponent<'a> {
    CreateComponent::ActionRow(CreateActionRow::buttons(vec![
        CreateButton::new("suggestions_accept")
            .label("Accept")
            .style(ButtonStyle::Success),
        CreateButton::new("suggestions_reject")
            .label("Reject")
            .style(ButtonStyle::Danger),
    ]))
}
