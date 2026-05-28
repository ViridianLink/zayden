use std::sync::Arc;

use serenity::all::{CommandInteraction, ComponentInteraction, Context, ModalInteraction};
use zayden_app::config::GuildConfig;
use zayden_app::state::AppState;

pub struct InvocationCtx<'a> {
    pub ctx: &'a Context,
    pub interaction: &'a CommandInteraction,
    pub app: Arc<AppState>,
    pub guild_config: Arc<GuildConfig>,
}

pub struct ComponentCtx<'a> {
    pub ctx: &'a Context,
    pub interaction: &'a ComponentInteraction,
    pub app: Arc<AppState>,
    pub guild_config: Arc<GuildConfig>,
}

pub struct ModalCtx<'a> {
    pub ctx: &'a Context,
    pub interaction: &'a ModalInteraction,
    pub app: Arc<AppState>,
    pub guild_config: Arc<GuildConfig>,
}

pub struct AutocompleteCtx<'a> {
    pub ctx: &'a Context,
    pub interaction: &'a CommandInteraction,
    pub app: Arc<AppState>,
    pub guild_config: Arc<GuildConfig>,
}
