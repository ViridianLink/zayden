use rand::rng;
use rand::seq::SliceRandom;
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateActionRow, CreateCommand,
    CreateCommandOption, EditInteractionResponse, ResolvedOption, ResolvedValue,
};
use sqlx::{Database, Pool};

use crate::games::blackjack::{double_button, hit_button, in_play_embed, stand_button};
use crate::models::gambling::GamblingManager;
use crate::{CARD_DECK, EffectsManager, GameCache, GameManager, GoalsManager, Result};

use super::Commands;

impl Commands {
    pub async fn blackjack<
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
        interaction.defer(ctx).await.unwrap();

        let Some(ResolvedValue::Integer(bet)) = options.pop().map(|opt| opt.value) else {
            unreachable!("bet is required")
        };

        let mut tx = pool.begin().await.unwrap();

        let coins = GamblingHandler::coins(&mut *tx, interaction.user.id)
            .await
            .unwrap();

        tx.commit().await.unwrap();

        GameCache::can_play(ctx, interaction.user.id).await?;
        EffectsHandler::bet_limit::<GamblingHandler>(pool, interaction.user.id, bet, coins).await?;
        GamblingHandler::bet(pool, interaction.user.id, bet)
            .await
            .unwrap();

        let mut card_shoe = CARD_DECK.to_vec();

        card_shoe.shuffle(&mut rng());

        let player_hand = vec![card_shoe.pop().unwrap(), card_shoe.pop().unwrap()];
        // let player_value = sum_cards(&player_hand);
        let dealer_hand = [card_shoe.pop().unwrap(), card_shoe.pop().unwrap()];
        // let dealer_value = sum_cards(&dealer_hand);

        // if player_value == 21 && dealer_value == 21 {
        //     Some(GameResult::Draw);
        // } else if player_value == 21 {
        //     Some(GameResult::Blackjack);
        // }

        let embed = in_play_embed(bet, &player_hand, dealer_hand[0]);

        let components = vec![CreateActionRow::Buttons(vec![
            hit_button(),
            stand_button(),
            double_button(coins, bet),
        ])];

        interaction
            .edit_response(
                ctx,
                EditInteractionResponse::new()
                    .embed(embed)
                    .components(components),
            )
            .await
            .unwrap();

        Ok(())
    }

    pub fn register_blackjack() -> CreateCommand {
        CreateCommand::new("blackjack")
            .description("Play a game of blackjack")
            .add_option(
                CreateCommandOption::new(CommandOptionType::Integer, "bet", "The amount to bet.")
                    .required(true),
            )
    }
}
