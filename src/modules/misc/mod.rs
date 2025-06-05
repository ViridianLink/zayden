pub use random::Random;
use serenity::all::{Context, CreateCommand};
use zayden_core::SlashCommand;

mod random;

pub fn register(ctx: &Context) -> CreateCommand {
    Random::register(ctx).unwrap()
}
