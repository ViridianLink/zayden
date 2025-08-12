pub use custom_msg::CustomMsg;
pub use random::Random;
use serenity::all::{Context, CreateCommand};
use zayden_core::ApplicationCommand;

mod custom_msg;
mod random;

pub fn register(ctx: &Context) -> [CreateCommand<'_>; 2] {
    [
        CustomMsg::register(ctx).unwrap(),
        Random::register(ctx).unwrap(),
    ]
}
