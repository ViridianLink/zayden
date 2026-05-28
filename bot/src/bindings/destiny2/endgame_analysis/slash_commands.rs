use std::borrow::Cow;

use async_trait::async_trait;
use endgame_analysis::{DimWishlistCommand, TierListCommand, WeaponCommand};
use serenity::all::CreateCommand;
use zayden_core::error::HandlerError;
use zayden_core::{AutocompleteCtx, InvocationCtx, ModuleAutocomplete, ModuleCommand};

use crate::BotState;

pub struct DimWishlist;

#[async_trait]
impl ModuleCommand for DimWishlist {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("dimwishlist")
    }

    fn definition(&self) -> CreateCommand<'static> {
        DimWishlistCommand::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let options = cx.interaction.data.options();
        DimWishlistCommand::run::<BotState>(cx.ctx, cx.interaction, options).await;
        Ok(())
    }
}

pub struct TierList;

#[async_trait]
impl ModuleCommand for TierList {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("tierlist")
    }

    fn definition(&self) -> CreateCommand<'static> {
        TierListCommand::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let options = cx.interaction.data.options();
        TierListCommand::run::<BotState>(cx.ctx, cx.interaction, options)
            .await
            .map_err(HandlerError::from_respond)
    }
}

#[async_trait]
impl ModuleAutocomplete for TierList {
    fn command(&self) -> Cow<'static, str> {
        Cow::Borrowed("tierlist")
    }

    async fn run(&self, cx: &AutocompleteCtx<'_>) -> Result<(), HandlerError> {
        if let Some(option) = cx.interaction.data.autocomplete() {
            TierListCommand::autocomplete::<BotState>(cx.ctx, cx.interaction, option)
                .await
                .map_err(HandlerError::from_respond)?;
        }
        Ok(())
    }
}

pub struct Weapon;

#[async_trait]
impl ModuleCommand for Weapon {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("weapon")
    }

    fn definition(&self) -> CreateCommand<'static> {
        WeaponCommand::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        WeaponCommand::run::<BotState>(cx.ctx, cx.interaction)
            .await
            .map_err(HandlerError::from_respond)
    }
}

#[async_trait]
impl ModuleAutocomplete for Weapon {
    fn command(&self) -> Cow<'static, str> {
        Cow::Borrowed("weapon")
    }

    async fn run(&self, cx: &AutocompleteCtx<'_>) -> Result<(), HandlerError> {
        if let Some(option) = cx.interaction.data.autocomplete() {
            WeaponCommand::autocomplete::<BotState>(cx.ctx, cx.interaction, option)
                .await
                .map_err(HandlerError::from_respond)?;
        }
        Ok(())
    }
}
