use rand::rng;
use rand::seq::SliceRandom;
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateActionRow, CreateCommand,
    CreateCommandOption, CreateComponent, EditInteractionResponse, ResolvedOption, ResolvedValue,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::EmojiCacheData;

use crate::games::blackjack::{
    GameDetails, double_button, game_end_blackjack, game_end_draw, hit_button, in_play_embed,
    stand_button, sum_cards,
};
use crate::models::gambling::GamblingManager;
use crate::{
    CARD_DECK, EffectsManager, GamblingData, GameCache, GameManager, GoalsManager, Result,
    card_deck,
};

use super::Commands;

impl Commands {
    pub async fn blackjack<
        Data: GamblingData + EmojiCacheData,
        Db: Database,
        GamblingHandler: GamblingManager<Db>,
        GoalsHandler: GoalsManager<Db>,
        EffectsHandler: EffectsManager<Db> + Send,
        GameHandler: GameManager<Db>,
    >(
        ctx: &Context,
        interaction: &CommandInteraction,
        mut options: Vec<ResolvedOption<'_>>,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let Some(ResolvedValue::Integer(bet)) = options.pop().map(|opt| opt.value) else {
            unreachable!("bet is required")
        };

        let mut tx = pool.begin().await.unwrap();

        let coins = GamblingHandler::coins(&mut *tx, interaction.user.id)
            .await
            .unwrap();

        tx.commit().await.unwrap();

        GameCache::can_play(ctx.data::<RwLock<Data>>(), interaction.user.id).await?;
        EffectsHandler::bet_limit::<GamblingHandler>(pool, interaction.user.id, bet, coins).await?;
        GamblingHandler::bet(pool, interaction.user.id, bet)
            .await
            .unwrap();

        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        let mut card_shoe = CARD_DECK.get_or_init(|| card_deck(&emojis)).to_vec();

        card_shoe.shuffle(&mut rng());

        let player_hand = vec![card_shoe.pop().unwrap(), card_shoe.pop().unwrap()];
        let player_value = sum_cards(&emojis, &player_hand);
        let dealer_hand = [card_shoe.pop().unwrap(), card_shoe.pop().unwrap()];
        let dealer_value = sum_cards(&emojis, &dealer_hand);

        let game = GameDetails::new(bet, player_hand, dealer_hand[0]);

        if player_value == 21 && dealer_value == 21 {
            let response = game_end_draw::<Data, Db, GoalsHandler, EffectsHandler, GameHandler>(
                ctx,
                pool,
                &emojis,
                interaction.user.id,
                interaction.channel_id,
                game,
                &dealer_hand,
            )
            .await;

            interaction
                .edit_response(&ctx.http, response)
                .await
                .unwrap();

            return Ok(());
        } else if player_value == 21 {
            let response =
                game_end_blackjack::<Data, Db, GoalsHandler, EffectsHandler, GameHandler>(
                    ctx,
                    pool,
                    &emojis,
                    interaction.user.id,
                    interaction.channel_id,
                    game,
                    &dealer_hand,
                )
                .await;

            interaction
                .edit_response(&ctx.http, response)
                .await
                .unwrap();

            return Ok(());
        }

        let embed = in_play_embed(&emojis, bet, game.player_hand(), dealer_hand[0]);

        let action_row = CreateComponent::ActionRow(CreateActionRow::buttons(vec![
            hit_button(),
            stand_button(),
            double_button(coins, bet),
        ]));

        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .embed(embed)
                    .components(vec![action_row]),
            )
            .await
            .unwrap();

        Ok(())
    }

    pub fn register_blackjack<'a>() -> CreateCommand<'a> {
        CreateCommand::new("blackjack")
            .description("Play a game of blackjack")
            .add_option(
                CreateCommandOption::new(CommandOptionType::Integer, "bet", "The amount to bet.")
                    .required(true),
            )
    }
}
