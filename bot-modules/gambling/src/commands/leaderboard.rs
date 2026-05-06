use serenity::all::{
    ButtonStyle, Colour, CommandInteraction, CommandOptionType, Context, CreateButton,
    CreateCommand, CreateCommandOption, CreateEmbed, CreateEmbedFooter, EditInteractionResponse,
    ResolvedOption, ResolvedValue,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::cache::GuildMembersCache;
use zayden_core::{EmojiCacheData, parse_options};

use crate::Result;
use crate::common::LeaderboardManager;
use crate::common::leaderboard::{get_row_number, get_rows};
use crate::shop::{EGGPLANT, LOTTO_TICKET};

use super::Commands;

impl Commands {
    pub async fn leaderboard<
        Data: GuildMembersCache + EmojiCacheData,
        Db: Database,
        Manager: LeaderboardManager<Db>,
    >(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await.unwrap();

        let mut options = parse_options(options);

        let ResolvedValue::String(leaderboard) = options.remove("leaderboard").unwrap() else {
            unreachable!("leaderboard option is required")
        };

        let global = match options.remove("global") {
            Some(ResolvedValue::Boolean(global)) => global,
            _ => false,
        };

        let users = if !global {
            let data = ctx.data::<RwLock<Data>>();
            let data = data.read().await;
            let users = data
                .get()
                .get(&interaction.guild_id.unwrap())
                .unwrap()
                .iter()
                .map(|id| id.get() as i64)
                .collect::<Vec<_>>();
            Some(users)
        } else {
            None
        };

        let rows = get_rows::<Db, Manager>(leaderboard, pool, users.as_deref(), 1).await;

        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        let desc = rows
            .into_iter()
            .enumerate()
            .map(|(i, row)| row.as_desc(&emojis, i))
            .collect::<Vec<_>>()
            .join("\n\n");

        let embed = CreateEmbed::new()
            .title(format!(
                "🏁 {}Leaderboard ({leaderboard})",
                if global { "Global " } else { "" }
            ))
            .description(desc)
            .footer(CreateEmbedFooter::new("Page 1"))
            .colour(Colour::TEAL);

        let mut response = EditInteractionResponse::new().embed(embed).button(
            CreateButton::new("leaderboard_previous")
                .label("<")
                .style(ButtonStyle::Secondary),
        );

        if get_row_number::<Db, Manager>(leaderboard, pool, users.as_deref(), interaction.user.id)
            .await
            .is_some()
        {
            response = response.button(
                CreateButton::new("leaderboard_user")
                    .emoji('🎯')
                    .style(ButtonStyle::Secondary),
            );
        }

        interaction
            .edit_response(
                &ctx.http,
                response.button(
                    CreateButton::new("leaderboard_next")
                        .label(">")
                        .style(ButtonStyle::Secondary),
                ),
            )
            .await
            .unwrap();

        Ok(())
    }

    pub fn register_leaderboard<'a>() -> CreateCommand<'a> {
        CreateCommand::new("leaderboard")
            .description("The server leaderboard")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "leaderboard",
                    "The leaderboard to choose",
                )
                .required(true)
                .add_string_choice("Coins", "coins")
                .add_string_choice("Gems", "gems")
                .add_string_choice(EGGPLANT.name, "eggplants")
                .add_string_choice(LOTTO_TICKET.name, "lottotickets")
                .add_string_choice("Higher or Lower", "higherlower")
                .add_string_choice("Weekly Higher or Lower", "weekly_higherlower"),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::Boolean,
                "global",
                "Whether to show global scores",
            ))
    }
}
