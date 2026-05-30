use std::sync::Arc;

use async_trait::async_trait;
use serenity::all::{
    CommandInteraction,
    ComponentInteraction,
    Context,
    EditInteractionResponse,
    Http,
    Interaction,
    Message,
    ModalInteraction,
};
use tracing::{error, warn};
use zayden_app::state::AppState;
use zayden_core::error::HandlerError;

mod autocomplete;
mod command;
mod component;
mod modal;

use super::Handler;
use crate::{CommandRegistry, Result};

#[async_trait]
pub(super) trait Respondable: Send + Sync {
    async fn defer_ephemeral_(&self, http: &Http) -> serenity::Result<()>;
    async fn send_error_reply(
        &self,
        http: &Http,
        content: &str,
    ) -> serenity::Result<Message>;
    fn interaction_name(&self) -> &str;
    fn user_name(&self) -> &str;
}

#[async_trait]
impl Respondable for CommandInteraction {
    async fn defer_ephemeral_(&self, http: &Http) -> serenity::Result<()> {
        self.defer_ephemeral(http).await
    }

    async fn send_error_reply(
        &self,
        http: &Http,
        content: &str,
    ) -> serenity::Result<Message> {
        self.edit_response(
            http,
            EditInteractionResponse::new().content(content.to_owned()),
        )
        .await
    }

    fn interaction_name(&self) -> &str {
        self.data.name.as_str()
    }

    fn user_name(&self) -> &str {
        self.user.name.as_str()
    }
}

#[async_trait]
impl Respondable for ComponentInteraction {
    async fn defer_ephemeral_(&self, http: &Http) -> serenity::Result<()> {
        self.defer_ephemeral(http).await
    }

    async fn send_error_reply(
        &self,
        http: &Http,
        content: &str,
    ) -> serenity::Result<Message> {
        self.edit_response(
            http,
            EditInteractionResponse::new().content(content.to_owned()),
        )
        .await
    }

    fn interaction_name(&self) -> &str {
        self.data.custom_id.as_str()
    }

    fn user_name(&self) -> &str {
        self.user.name.as_str()
    }
}

#[async_trait]
impl Respondable for ModalInteraction {
    async fn defer_ephemeral_(&self, http: &Http) -> serenity::Result<()> {
        self.defer_ephemeral(http).await
    }

    async fn send_error_reply(
        &self,
        http: &Http,
        content: &str,
    ) -> serenity::Result<Message> {
        self.edit_response(
            http,
            EditInteractionResponse::new().content(content.to_owned()),
        )
        .await
    }

    fn interaction_name(&self) -> &str {
        self.data.custom_id.as_str()
    }

    fn user_name(&self) -> &str {
        self.user.name.as_str()
    }
}

pub(super) async fn respond_with_error(
    ctx: &Context,
    interaction: &impl Respondable,
    err: HandlerError,
) {
    match err.user_message() {
        Some(msg) => {
            let _ = interaction.defer_ephemeral_(&ctx.http).await;
            if let Err(send_err) = interaction.send_error_reply(&ctx.http, msg).await
            {
                error!(
                    error = ?err,
                    send_err = ?send_err,
                    name = interaction.interaction_name(),
                    user = interaction.user_name(),
                    "failed to deliver user error message",
                );
            }
        },
        None => {
            error!(
                error = ?err,
                name = interaction.interaction_name(),
                user = interaction.user_name(),
                "internal error in interaction handler",
            );
        },
    }
}

impl Handler {
    pub async fn interaction_create(
        ctx: &Context,
        interaction: &Interaction,
        app: Arc<AppState>,
        registry: Arc<CommandRegistry>,
    ) -> Result<()> {
        match interaction {
            Interaction::Command(command) => {
                Self::interaction_command(
                    ctx,
                    command,
                    Arc::clone(&app),
                    Arc::clone(&registry),
                )
                .await;
                Ok(())
            },
            Interaction::Autocomplete(autocomplete) => {
                Self::interaction_autocomplete(
                    ctx,
                    autocomplete,
                    Arc::clone(&app),
                    Arc::clone(&registry),
                )
                .await
            },
            Interaction::Component(component) => {
                Self::interaction_component(
                    ctx,
                    component,
                    Arc::clone(&app),
                    Arc::clone(&registry),
                )
                .await
            },
            Interaction::Modal(modal) => {
                Self::interaction_modal(
                    ctx,
                    modal,
                    Arc::clone(&app),
                    Arc::clone(&registry),
                )
                .await
            },
            Interaction::Ping(_) => Ok(()),
            other => {
                warn!(kind = ?other.kind(), "interaction kind not handled");
                Ok(())
            },
        }
    }
}
