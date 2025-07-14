use serenity::all::{Context, CreateCommand};
use zayden_core::SlashCommand;

mod blackjack;
mod coinflip;
mod craft;
mod daily;
mod dig;
mod effects;
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
pub use craft::Craft;
pub use daily::Daily;
pub use dig::Dig;
pub use effects::EffectsTable;
pub use gift::Gift;
pub use goals::{Goals, GoalsTable};
pub use higher_lower::{HigherLower, HigherLowerTable};
pub use inventory::Inventory;
pub use leaderboard::Leaderboard;
pub use lotto::{Lotto, LottoTable};
pub use mine::{Mine, MineTable};
pub use models::{GamblingTable, GameTable, StatsTable};
pub use prestige::Prestige;
pub use profile::Profile;
pub use roll::Roll;
pub use rps::RockPaperScissors;
pub use send::Send;
pub use shop::Shop;
pub use stamina::StaminaTable;
pub use tictactoe::TicTacToe;
pub use work::Work;

pub fn register(ctx: &Context) -> [CreateCommand; 20] {
    [
        Blackjack::register(ctx).unwrap(),
        Coinflip::register(ctx).unwrap(),
        Craft::register(ctx).unwrap(),
        Daily::register(ctx).unwrap(),
        Dig::register(ctx).unwrap(),
        Gift::register(ctx).unwrap(),
        Goals::register(ctx).unwrap(),
        HigherLower::register(ctx).unwrap(),
        Inventory::register(ctx).unwrap(),
        Leaderboard::register(ctx).unwrap(),
        Lotto::register(ctx).unwrap(),
        Mine::register(ctx).unwrap(),
        Prestige::register(ctx).unwrap(),
        Profile::register(ctx).unwrap(),
        Roll::register(ctx).unwrap(),
        RockPaperScissors::register(ctx).unwrap(),
        Send::register(ctx).unwrap(),
        Shop::register(ctx).unwrap(),
        TicTacToe::register(ctx).unwrap(),
        Work::register(ctx).unwrap(),
    ]
}
