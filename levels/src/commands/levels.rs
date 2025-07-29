use std::time::Duration;

use futures::StreamExt;
use serenity::all::{
    CollectComponentInteractions, CommandInteraction, ComponentInteraction, Context, CreateButton,
    CreateCommand, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage, EditInteractionResponse, GuildId, Mentionable,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::cache::GuildMembersCache;

use crate::{LeaderboardRow, LevelsManager, LevelsRow};

pub struct Levels;

impl Levels {
    pub async fn run<Data: GuildMembersCache, Db: Database, Manager: LevelsManager<Db>>(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) {
        interaction.defer(&ctx.http).await.unwrap();

        let embed =
            create_embed::<Data, Db, Manager>(ctx, pool, interaction.guild_id.unwrap(), 1).await;

        let msg = interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .embed(embed)
                    .button(CreateButton::new("previous").label("<"))
                    .button(CreateButton::new("user").emoji('🎯'))
                    .button(CreateButton::new("next").label(">")),
            )
            .await
            .unwrap();

        let mut stream = msg
            .id
            .collect_component_interactions(ctx)
            .timeout(Duration::from_secs(120))
            .stream();

        while let Some(component) = stream.next().await {
            run_components::<Data, Db, Manager>(ctx, component, pool).await
        }
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("levels").description("Get the leaderboard")
    }
}

async fn run_components<Data: GuildMembersCache, Db: Database, Manager: LevelsManager<Db>>(
    ctx: &Context,
    interaction: ComponentInteraction,
    pool: &Pool<Db>,
) {
    let Some(embed) = interaction.message.embeds.first() else {
        unreachable!("Embed must be present")
    };

    let page_number = match interaction.data.custom_id.as_str() {
        "previous" => {
            embed
                .footer
                .as_ref()
                .unwrap()
                .text
                .strip_prefix("Page ")
                .unwrap()
                .parse::<i64>()
                .unwrap()
                - 1
        }
        "user" => {
            let Some(row_number) = Manager::user_rank(pool, interaction.user.id).await.unwrap()
            else {
                return;
            };

            row_number / 10 + 1
        }
        "next" => {
            embed
                .footer
                .as_ref()
                .unwrap()
                .text
                .strip_prefix("Page ")
                .unwrap()
                .parse::<i64>()
                .unwrap()
                + 1
        }
        _ => unreachable!(),
    }
    .max(1);

    let embed = create_embed::<Data, Db, Manager>(
        ctx,
        pool,
        interaction.guild_id.unwrap(),
        page_number + 1,
    )
    .await;

    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().embed(embed),
            ),
        )
        .await
        .unwrap();
}

async fn create_embed<'a, Data: GuildMembersCache, Db: Database, Manager: LevelsManager<Db>>(
    ctx: &Context,
    pool: &Pool<Db>,
    guild_id: GuildId,
    page_number: i64,
) -> CreateEmbed<'a> {
    let users = {
        let data = ctx.data::<RwLock<Data>>();
        let data = data.read().await;

        data.get()
            .get(&guild_id)
            .unwrap()
            .iter()
            .map(|id| id.get() as i64)
            .collect::<Vec<_>>()
    };

    let rows = Manager::leaderboard(pool, &users, page_number)
        .await
        .unwrap();

    let desc = rows
        .into_iter()
        .enumerate()
        .map(|(i, row)| row_as_desc(&row, i + (page_number as usize - 1) * 10))
        .collect::<Vec<_>>()
        .join("\n\n");

    CreateEmbed::new()
        .title("Leaderboard")
        .description(desc)
        .footer(CreateEmbedFooter::new(format!("Page {page_number}")))
}

fn row_as_desc(row: &LeaderboardRow, i: usize) -> String {
    let place = if i == 0 {
        "🥇".to_string()
    } else if i == 1 {
        "🥈".to_string()
    } else if i == 2 {
        "🥉".to_string()
    } else {
        format!("#{}", i + 1)
    };

    let data = format!(
        "{}\n(Messages: {} | Total XP: {})",
        row.level(),
        row.message_count(),
        row.xp(),
    );

    format!("{place} - {} - {data}", row.user_id().mention())
}
