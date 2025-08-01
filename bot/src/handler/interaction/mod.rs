use serenity::all::{Context, Interaction};
use sqlx::PgPool;

mod autocomplete;
mod command;
mod component;
mod modal;

use crate::Result;

use super::Handler;

impl Handler {
    pub async fn interaction_create(
        ctx: &Context,
        interaction: &Interaction,
        pool: &PgPool,
    ) -> Result<()> {
        match interaction {
            Interaction::Command(command) => Handler::interaction_command(ctx, command, pool).await,
            Interaction::Autocomplete(autocomplete) => {
                Handler::interaction_autocomplete(ctx, autocomplete, pool).await
            }
            Interaction::Component(component) => {
                Handler::interaction_component(ctx, component, pool).await
            }
            Interaction::Modal(modal) => Handler::interaction_modal(ctx, modal, pool).await,
            _ => unimplemented!("Interaction not implemented: {:?}", interaction.kind()),
        }
    }
}
