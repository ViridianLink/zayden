pub mod ai;

pub mod destiny2;

pub mod events;

pub mod gambling;

pub mod levels;

pub mod lfg;

pub mod llamad2;

pub mod misc;

pub mod reaction_roles;

pub mod suggestions;

pub mod temp_voice;

pub mod ticket;

pub mod verify;

pub fn build_registry() -> std::sync::Arc<crate::CommandRegistry> {
    let mut builder = crate::RegistryBuilder::new();
    ai::register(&mut builder);
    destiny2::register(&mut builder);
    events::register(&mut builder);
    gambling::register(&mut builder);
    lfg::register(&mut builder);
    levels::register(&mut builder);
    llamad2::register(&mut builder);
    misc::register(&mut builder);
    ticket::register(&mut builder);
    verify::register(&mut builder);
    suggestions::register(&mut builder);
    temp_voice::register(&mut builder);
    reaction_roles::register(&mut builder);
    builder.build()
}
