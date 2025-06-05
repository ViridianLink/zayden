use serenity::all::{Context, CreateCommand};

pub mod levels;
use levels::Commands;

// pub mod moderation;
pub mod ai;
pub mod destiny2;
pub mod events;
pub mod gambling;
pub mod misc;
pub mod reaction_roles;
pub mod suggestions;
pub mod temp_voice;
pub mod ticket;

pub fn global_register(ctx: &Context) -> Vec<CreateCommand> {
    let mut cmds = destiny2::register(ctx).to_vec();

    cmds.extend(gambling::register(ctx));
    cmds.extend(Commands::register());
    cmds.push(misc::register(ctx));
    cmds.push(reaction_roles::register(ctx));
    cmds.push(suggestions::register(ctx));
    cmds.push(temp_voice::register(ctx));
    cmds.extend(ticket::register(ctx));

    cmds
}
