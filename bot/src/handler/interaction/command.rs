use chrono::Utc;
use serenity::all::{
    CommandInteraction, Context, CreateInteractionResponse, CreateInteractionResponseMessage,
    EditInteractionResponse,
};
use sqlx::PgPool;
use zayden_core::{ApplicationCommand, get_option_str};

use crate::Result;
use crate::handler::Handler;
use crate::modules::destiny2::Perk;
use crate::modules::destiny2::endgame_analysis::slash_commands::{DimWishlist, TierList, Weapon};
use crate::modules::destiny2::loadouts::Loadout;
use crate::modules::destiny2::raid_guide::RaidGuide;
use crate::modules::events::live::Live;
use crate::modules::gambling::{
    Blackjack, Coinflip, Craft, Daily, Dig, Gift, Goals, HigherLower, Inventory, Leaderboard,
    Lotto, Mine, Prestige, Profile, RockPaperScissors, Roll, Send, Shop, TicTacToe, Work,
};
use crate::modules::levels::{Levels, Rank, Xp};
use crate::modules::lfg::Lfg;
use crate::modules::misc::{CustomMsg, Random};
use crate::modules::music::Play;
use crate::modules::reaction_roles::ReactionRoleCommand;
use crate::modules::suggestions::FetchSuggestions;
use crate::modules::temp_voice::Voice;
use crate::modules::ticket::slash_commands::{SupportCommand, TicketCommand};
use crate::modules::verify::Panel;

impl Handler {
    pub async fn interaction_command(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        let options = interaction.data.options();

        println!(
            "[{}] {} ran command: {}{}",
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            interaction.user.name,
            interaction.data.name,
            get_option_str(&options)
        );

        let result = match interaction.data.name.to_ascii_lowercase().as_str() {
            // region Destiny 2
            "weapon" => Weapon::run(ctx, interaction, options, pool),
            "dimwishlist" => DimWishlist::run(ctx, interaction, options, pool),
            "lfg" => Lfg::run(ctx, interaction, options, pool),
            "tierlist" => TierList::run(ctx, interaction, options, pool),
            "perk" => Perk::run(ctx, interaction, options, pool),
            // endregion

            // region gambling
            "blackjack" => Blackjack::run(ctx, interaction, options, pool),
            "coinflip" => Coinflip::run(ctx, interaction, options, pool),
            "craft" => Craft::run(ctx, interaction, options, pool),
            "daily" => Daily::run(ctx, interaction, options, pool),
            "dig" => Dig::run(ctx, interaction, options, pool),
            "inventory" => Inventory::run(ctx, interaction, options, pool),
            "higherorlower" => HigherLower::run(ctx, interaction, options, pool),
            "leaderboard" => Leaderboard::run(ctx, interaction, options, pool),
            "lotto" => Lotto::run(ctx, interaction, options, pool),
            "mine" => Mine::run(ctx, interaction, options, pool),
            "profile" => Profile::run(ctx, interaction, options, pool),
            "prestige" => Prestige::run(ctx, interaction, options, pool),
            "rps" => RockPaperScissors::run(ctx, interaction, options, pool),
            "roll" => Roll::run(ctx, interaction, options, pool),
            "work" => Work::run(ctx, interaction, options, pool),
            "gift" => Gift::run(ctx, interaction, options, pool),
            "goals" => Goals::run(ctx, interaction, options, pool),
            "send" => Send::run(ctx, interaction, options, pool),
            "shop" => Shop::run(ctx, interaction, options, pool),
            "tictactoe" => TicTacToe::run(ctx, interaction, options, pool),
            // endregion

            //region music
            "play" => Play::run(ctx, interaction, options, pool),
            //endregion
            "levels" => Levels::run(ctx, interaction, options, pool),
            "random" => Random::run(ctx, interaction, options, pool),
            "fetch_suggestions" => FetchSuggestions::run(ctx, interaction, options, pool),
            "live" => Live::run(ctx, interaction, options, pool),
            "rank" => Rank::run(ctx, interaction, options, pool),
            "xp" => Xp::run(ctx, interaction, options, pool),
            "reaction_role" => ReactionRoleCommand::run(ctx, interaction, options, pool),
            "voice" => Voice::run(ctx, interaction, options, pool),
            "raidguide" => RaidGuide::run(ctx, interaction, options, pool),
            "builds" => Loadout::run(ctx, interaction, options, pool),
            "custom_msg" => CustomMsg::run(ctx, interaction, options, pool),
            "panel" => Panel::run(ctx, interaction, options, pool),

            // region: ticket
            "ticket" => TicketCommand::run(ctx, interaction, options, pool),
            "support" => SupportCommand::run(ctx, interaction, options, pool),
            // endregion: ticket
            _ => {
                println!("Unknown command: {}", interaction.data.name);
                return Ok(());
            }
        }
        .await;

        if let Err(e) = result
            && interaction
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content(e.to_string()),
                    ),
                )
                .await
                .is_err()
        {
            interaction
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new().content(e.to_string()),
                )
                .await?;
        }

        Ok(())
    }
}
