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
mod prestige;
mod profile;
mod roll;
mod rps;
mod send;
mod shop;
mod tictactoe;
mod work;

pub use blackjack::Blackjack;
pub use coinflip::Coinflip;
pub use craft::Craft;
pub use daily::Daily;
pub use dig::Dig;
pub use gift::Gift;
pub use goals::Goals;
pub use higher_lower::HigherLower;
pub use inventory::Inventory;
pub use leaderboard::Leaderboard;
pub use lotto::Lotto;
pub use mine::Mine;
pub use prestige::Prestige;
pub use profile::Profile;
pub use roll::Roll;
pub use rps::RockPaperScissors;
pub use send::Send;
pub use shop::Shop;
pub use tictactoe::TicTacToe;
pub use work::Work;

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
