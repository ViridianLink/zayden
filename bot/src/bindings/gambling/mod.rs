mod blackjack;
mod coinflip;
mod craft;
mod daily;
mod dig;
mod gift;
mod goals;
mod higher_lower;
mod inventory;
mod leaderboard;
mod lotto;
mod mine;
mod models;
mod prestige;
mod profile;
mod roll;
mod rps;
mod send;
mod shop;
mod stamina;
mod tictactoe;
mod work;

pub use blackjack::Blackjack;
pub use coinflip::Coinflip;
pub use craft::{Craft, CraftTable};
pub use daily::{Daily, DailyTable};
pub use dig::{Dig, DigTable};
pub use gift::{Gift, GiftTable};
pub use goals::{Goals, GoalsTable};
pub use higher_lower::{HigherLower, HigherLowerTable};
pub use inventory::{Inventory, InventoryTable};
pub use leaderboard::{Leaderboard, LeaderboardTable};
pub use lotto::{Lotto, LottoTable};
pub use mine::{Mine, MineTable};
pub use models::{GamblingTable, GameTable, StatsTable};
pub use prestige::{Prestige, PrestigeTable};
pub use profile::{Profile, ProfileTable};
pub use roll::Roll;
pub use rps::RockPaperScissors;
pub use send::{Send, SendTable};
pub use shop::{Shop, ShopTable};
pub use stamina::StaminaTable;
pub use tictactoe::TicTacToe;
pub use work::{Work, WorkTable};

use crate::RegistryBuilder;
use crate::registry::OverlapError;

pub fn register(builder: &mut RegistryBuilder) -> Result<(), OverlapError> {
    builder
        .add_command(Blackjack)
        .add_component(Blackjack)?
        .add_command(Coinflip)
        .add_command(Craft)
        .add_command(Daily)
        .add_command(Dig)
        .add_command(Gift)
        .add_command(Goals)
        .add_command(HigherLower)
        .add_component(HigherLower)?
        .add_command(Inventory)
        .add_command(Leaderboard)
        .add_component(Leaderboard)?
        .add_command(Lotto)
        .add_command(Mine)
        .add_command(Prestige)
        .add_component(Prestige)?
        .add_command(Profile)
        .add_command(Roll)
        .add_command(RockPaperScissors)
        .add_command(Send)
        .add_command(Shop)
        .add_command(TicTacToe)
        .add_component(TicTacToe)?
        .add_command(Work);

    Ok(())
}
