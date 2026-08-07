pub mod blackjack;
pub mod higherlower;
pub mod lotto;
pub mod tiktactoe;

pub use higherlower::{HigherLower, HigherLowerManager};
pub use lotto::{Lotto, LottoManager, LottoRow, jackpot, select_winners};
