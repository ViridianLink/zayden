use std::collections::HashMap;
use std::sync::LazyLock;

use sqlx::Postgres;
use zayden_core::{ApplicationCommand, application_commands};

use crate::Error;
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
use crate::modules::llamad2::{
    DungeonReport, Goof, Hello, Playlist, RaidReport, Sensitivity, Socials,
};
use crate::modules::misc::{CustomMsg, Random};
use crate::modules::reaction_roles::ReactionRoleCommand;
use crate::modules::suggestions::FetchSuggestions;
use crate::modules::temp_voice::Voice;
use crate::modules::ticket::slash_commands::{SupportCommand, TicketCommand};
use crate::modules::verify::Panel;

pub static APPLICATION_COMMANDS: LazyLock<
    HashMap<String, Box<dyn ApplicationCommand<Error, Postgres>>>,
> = LazyLock::new(|| {
    application_commands! {
        Error, Postgres;

        Weapon,
        DimWishlist,
        Lfg,
        TierList,
        Perk,

        Blackjack,
        Coinflip,
        Craft,
        Daily,
        Dig,
        Inventory,
        HigherLower,
        Leaderboard,
        Lotto,
        Mine,
        Profile,
        Prestige,
        RockPaperScissors,
        Roll,
        Work,
        Gift,
        Goals,
        Send,
        Shop,
        TicTacToe,

        Levels,
        Rank,
        Xp,

        DungeonReport,
        Goof,
        Hello,
        Playlist,
        RaidReport,
        Sensitivity,
        Socials,
        Random,
        FetchSuggestions,
        Live,
        ReactionRoleCommand,
        Voice,
        RaidGuide,
        Loadout,
        CustomMsg,
        Panel,

        TicketCommand,
        SupportCommand,
    }
});
