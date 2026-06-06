use serenity::all::{
    ButtonStyle,
    Colour,
    CommandInteraction,
    CommandOptionType,
    Context,
    CreateButton,
    CreateCommand,
    CreateCommandOption,
    CreateEmbed,
    CreateEmbedFooter,
    EditInteractionResponse,
    ResolvedOption,
    ResolvedValue,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::cache::GuildMembersCache;
use zayden_core::{EmojiCacheData, as_i64, parse_options};

use super::Commands;
use crate::Result;
use crate::common::LeaderboardManager;
use crate::common::leaderboard::{get_row_number, get_rows};
use crate::shop::{EGGPLANT, LOTTO_TICKET};

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
        interaction.defer(&ctx.http).await?;

        let mut options = parse_options(options);

        let Some(ResolvedValue::String(leaderboard)) = options.remove("leaderboard")
        else {
            return Err(crate::GamblingError::InvalidAmount);
        };

        let global = match options.remove("global") {
            Some(ResolvedValue::Boolean(global)) => global,
            _ => false,
        };

        let users = if global {
            None
        } else {
            let users = {
                let data = ctx.data::<RwLock<Data>>();
                let data = data.read().await;
                data.get()
                    .get(
                        &interaction
                            .guild_id
                            .expect("gambling command always used in guild"),
                    )
                    .expect("guild members cached when leaderboard command ran")
                    .iter()
                    .map(|id| as_i64(id.get()))
                    .collect::<Vec<_>>()
            };
            Some(users)
        };

        let rows =
            get_rows::<Db, Manager>(leaderboard, pool, users.as_deref(), 1).await;

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

        if get_row_number::<Db, Manager>(
            leaderboard,
            pool,
            users.as_deref(),
            interaction.user.id,
        )
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
            .await?;

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
