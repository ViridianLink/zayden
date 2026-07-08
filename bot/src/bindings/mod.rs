use std::sync::Arc;

use crate::registry::OverlapError;
use crate::{CommandRegistry, RegistryBuilder};

pub mod ai;
pub mod destiny2;
pub mod family;
pub mod gambling;
pub mod gold_star;
pub mod levels;
pub mod lfg;
pub mod llamad2;
pub mod marathon;
pub mod misc;
pub mod music;
pub mod reaction_roles;
pub mod suggestions;
pub mod temp_voice;
pub mod ticket;
pub mod verify;

pub fn build_registry(
    llamad2_guild: u64,
) -> Result<Arc<CommandRegistry>, OverlapError> {
    let mut builder = RegistryBuilder::new();
    ai::register(&mut builder);
    destiny2::register(&mut builder);
    family::register(&mut builder)?;
    gambling::register(&mut builder)?;
    gold_star::register(&mut builder);
    lfg::register(&mut builder)?;
    levels::register(&mut builder)?;
    llamad2::register(&mut builder, llamad2_guild);
    marathon::register(&mut builder);
    misc::register(&mut builder);
    music::register(&mut builder)?;
    ticket::register(&mut builder)?;
    verify::register(&mut builder)?;
    suggestions::register(&mut builder)?;
    temp_voice::register(&mut builder);
    reaction_roles::register(&mut builder);

    Ok(builder.build())
}
