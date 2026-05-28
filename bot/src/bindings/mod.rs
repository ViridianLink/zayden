pub mod ai;

pub mod destiny2;
use destiny2::Perk;
use destiny2::endgame_analysis::slash_commands::{DimWishlist, TierList, Weapon};
use destiny2::loadouts::Loadout;
use destiny2::raid_guide::RaidGuide;

pub mod events;
use events::Live;

pub mod gambling;
use gambling::{
    Blackjack, Coinflip, Craft, Daily, Dig, Gift, Goals, HigherLower, Inventory, Leaderboard,
    Lotto, Mine, Prestige, Profile, RockPaperScissors, Roll, Send, Shop, TicTacToe, Work,
};

pub mod levels;

pub mod lfg;
use lfg::Lfg;

pub mod llamad2;
use llamad2::{DungeonReport, Goof, Hello, Playlist, RaidReport, Sensitivity, Socials};

pub mod misc;
use misc::{CustomMsg, Random};

pub mod reaction_roles;
use reaction_roles::ReactionRoleCommand;

pub mod suggestions;
use suggestions::FetchSuggestions;

pub mod temp_voice;
use temp_voice::Voice;

pub mod ticket;
use ticket::slash_commands::{SupportCommand, TicketCommand};

pub mod verify;
use verify::Panel;

use std::collections::HashMap;
use std::sync::LazyLock;

use sqlx::Postgres;
use zayden_core::{ApplicationCommand, application_commands};

use crate::Error;

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

pub fn build_registry() -> std::sync::Arc<crate::CommandRegistry> {
    let mut builder = crate::RegistryBuilder::new();
    levels::register(&mut builder);
    builder.build()
}
