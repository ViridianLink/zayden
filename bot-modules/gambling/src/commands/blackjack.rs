use rand::rng;
use rand::seq::SliceRandom;
use serenity::all::{
    Colour,
    CommandInteraction,
    CommandOptionType,
    Context,
    CreateActionRow,
    CreateCommand,
    CreateCommandOption,
    CreateComponent,
    CreateContainer,
    CreateContainerComponent,
    EditInteractionResponse,
    MessageFlags,
    ResolvedOption,
    ResolvedValue,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::EmojiCacheData;

use super::Commands;
use crate::games::blackjack::{
    GameDetails,
    double_button,
    game_end_blackjack,
    game_end_draw,
    hit_button,
    in_play_text,
    split_button,
    stand_button,
    sum_cards,
    surrender_button,
};
use crate::models::gambling::GamblingManager;
use crate::{
    CARD_DECK,
    EffectsManager,
    GamblingData,
    GamblingError,
    GameManager,
    GoalsManager,
    Result,
    card_deck,
};

impl Commands {
    pub async fn blackjack<
        Data: GamblingData + EmojiCacheData,
        Db: Database,
        GamblingHandler: GamblingManager<Db>,
        GoalsHandler: GoalsManager<Db> + Send + Sync,
        EffectsHandler: EffectsManager<Db> + Send,
        GameHandler: GameManager<Db>,
    >(
        ctx: &Context,
        interaction: &CommandInteraction,
        mut options: Vec<ResolvedOption<'_>>,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let Some(ResolvedValue::Integer(bet)) = options.pop().map(|opt| opt.value)
        else {
            return Err(GamblingError::InvalidAmount);
        };

        let mut tx = pool.begin().await?;

        let coins = GamblingHandler::coins(&mut *tx, interaction.user.id).await?;

        tx.commit().await?;

        ctx.data::<RwLock<Data>>()
            .read()
            .await
            .game_cache()
            .check_and_set(interaction.user.id)?;
        EffectsHandler::bet_limit::<GamblingHandler>(
            pool,
            interaction.user.id,
            bet,
            coins,
        )
        .await?;
        GamblingHandler::bet(pool, interaction.user.id, bet).await?;

        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        let mut card_shoe = CARD_DECK.get_or_init(|| card_deck(&emojis)).clone();

        card_shoe.shuffle(&mut rng());

        let player_hand = vec![
            card_shoe.pop().expect("card shoe not empty"),
            card_shoe.pop().expect("card shoe not empty"),
        ];
        let player_value = sum_cards(&emojis, &player_hand);
        let dealer_hand = [
            card_shoe.pop().expect("card shoe not empty"),
            card_shoe.pop().expect("card shoe not empty"),
        ];
        let dealer_value = sum_cards(&emojis, &dealer_hand);

        let game = GameDetails::new(bet, player_hand, dealer_hand[0]);

        if player_value == 21 && dealer_value == 21 {
            let response =
                game_end_draw::<Db, GoalsHandler, EffectsHandler, GameHandler>(
                    ctx,
                    pool,
                    &emojis,
                    interaction.user.id,
                    interaction.channel_id,
                    game,
                    &dealer_hand,
                )
                .await?;

            interaction.edit_response(&ctx.http, response).await?;

            return Ok(());
        } else if player_value == 21 {
            let response =
                game_end_blackjack::<Db, GoalsHandler, EffectsHandler, GameHandler>(
                    ctx,
                    pool,
                    &emojis,
                    interaction.user.id,
                    interaction.channel_id,
                    game,
                    &dealer_hand,
                )
                .await?;

            interaction.edit_response(&ctx.http, response).await?;

            return Ok(());
        }

        let text = in_play_text(&emojis, bet, game.player_hand(), dealer_hand[0]);

        let action_row =
            CreateContainerComponent::ActionRow(CreateActionRow::buttons(vec![
                hit_button(),
                stand_button(),
                double_button().disabled(coins < bet * 2),
                split_button().disabled(true), //.disabled(coins < bet * 2),
                surrender_button(),
            ]));

        let container = CreateComponent::Container(
            CreateContainer::new(vec![text, action_row]).accent_colour(Colour::TEAL),
        );

        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .flags(MessageFlags::IS_COMPONENTS_V2)
                    .components(vec![container]),
            )
            .await?;

        Ok(())
    }

    pub fn register_blackjack<'a>() -> CreateCommand<'a> {
        CreateCommand::new("blackjack")
            .description("Play a game of blackjack")
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
