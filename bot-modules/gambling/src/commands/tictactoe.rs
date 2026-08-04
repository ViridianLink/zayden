use serenity::all::{
    ButtonStyle,
    CommandInteraction,
    CommandOptionType,
    Context,
    CreateButton,
    CreateCommand,
    CreateCommandOption,
    CreateEmbed,
    EditInteractionResponse,
    Mentionable,
    ResolvedOption,
    ResolvedValue,
};
use sqlx::PgPool;
use tokio::sync::RwLock;
use zayden_core::{EmojiCacheData, parse_options};

use super::Commands;
use crate::components::TicTacToeCustomId;
use crate::{Coins, EffectsManager, GamblingData, GamblingError, GameRow, Result};

impl Commands {
    pub async fn tictactoe<Data: GamblingData + EmojiCacheData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let row = GameRow::get(pool, interaction.user.id)
            .await?
            .unwrap_or_else(|| GameRow::new(interaction.user.id));

        let data_lock = ctx.data::<RwLock<Data>>();

        data_lock.read().await.game_cache().check_and_set(interaction.user.id)?;

        let mut options = parse_options(options);

        let Some(ResolvedValue::String(size)) = options.remove("size") else {
            return Err(GamblingError::InvalidAmount);
        };

        let Some(ResolvedValue::Integer(bet)) = options.remove("bet") else {
            return Err(GamblingError::InvalidAmount);
        };

        EffectsManager::bet_limit(pool, interaction.user.id, bet, row.coins())
            .await?;

        GameRow::ensure(pool, interaction.user.id).await?;

        let coin = data_lock.read().await.emojis().emoji("heads").map_err(|n| {
            GamblingError::Internal(format!("emoji '{n}' not in cache"))
        })?;

        let embed = CreateEmbed::new().title("TicTacToe").description(format!(
            "{} wants to play tic-tac-toe ({size}x{size}) for **{bet}** <:coin:{coin}>",
            interaction.user.mention(),
        ));

        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .embed(embed.clone())
                    .button(
                        CreateButton::new(TicTacToeCustomId::Accept.to_string())
                            .label("Accept")
                            .emoji('✅')
                            .style(ButtonStyle::Secondary),
                    )
                    .button(
                        CreateButton::new(TicTacToeCustomId::Cancel.to_string())
                            .label("Cancel")
                            .emoji('❌')
                            .style(ButtonStyle::Secondary),
                    ),
            )
            .await?;

        Ok(())
    }

    pub fn register_tictactoe<'a>() -> CreateCommand<'a> {
        CreateCommand::new("tictactoe")
            .description("Play a game of tic tac toe")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "size",
                    "Choose the board size to play.",
                )
                .add_string_choice("3x3", "3")
                .add_string_choice("4x4", "4")
                .add_string_choice("5x5", "5")
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "bet",
                    "The amount to bet.",
                )
                .required(true),
            )
    }
}
