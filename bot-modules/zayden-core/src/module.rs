use std::borrow::Cow;

use async_trait::async_trait;
use serenity::all::CreateCommand;

use crate::ctx::{AutocompleteCtx, ComponentCtx, InvocationCtx, ModalCtx};
use crate::error::HandlerError;
use crate::scope::{CommandMetadata, CommandScope, IdMatch};

#[async_trait]
pub trait ModuleCommand: Send + Sync {
    /// The command name, acquired at runtime so the same trait can be reused
    /// across multiple bot applications without hard-coding.
    fn name(&self) -> Cow<'static, str>;

    fn definition(&self) -> CreateCommand<'_>;

    fn scope(&self) -> CommandScope {
        CommandScope::Global
    }

    fn metadata(&self) -> CommandMetadata {
        CommandMetadata::default()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError>;
}

#[async_trait]
pub trait ModuleComponent: Send + Sync {
    fn id_match(&self) -> IdMatch;
    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError>;
}

#[async_trait]
pub trait ModuleModal: Send + Sync {
    fn id_match(&self) -> IdMatch;
    async fn run(&self, cx: &ModalCtx<'_>) -> Result<(), HandlerError>;
}

#[async_trait]
pub trait ModuleAutocomplete: Send + Sync {
    fn command(&self) -> Cow<'static, str>;
    async fn run(&self, cx: &AutocompleteCtx<'_>) -> Result<(), HandlerError>;
}
