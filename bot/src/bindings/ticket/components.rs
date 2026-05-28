use std::borrow::Cow;

use async_trait::async_trait;
use serenity::all::{
    CreateInputText, CreateLabel, CreateModalComponent, CreateSelectMenu, CreateSelectMenuKind,
    CreateSelectMenuOption, InputTextStyle,
};
use sqlx::Postgres;
use ticket::{TicketComponent, TicketModal};
use zayden_core::ctx::{ComponentCtx, ModalCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{ModuleComponent, ModuleModal};
use zayden_core::scope::IdMatch;

use crate::sqlx_lib::GuildTable;

use super::TicketTable;

fn make_ticket_modal_components<'a>() -> Vec<CreateModalComponent<'a>> {
    vec![
        CreateModalComponent::Label(CreateLabel::select_menu(
            "Select a relevent category:",
            CreateSelectMenu::new(
                "ticket_category",
                CreateSelectMenuKind::String {
                    options: vec![
                        CreateSelectMenuOption::new("Complaint", "complaint"),
                        CreateSelectMenuOption::new("General question", "general_question"),
                        CreateSelectMenuOption::new("Role Application", "role_application"),
                        CreateSelectMenuOption::new("Suggestion", "suggestion"),
                    ]
                    .into(),
                },
            ),
        )),
        CreateModalComponent::Label(CreateLabel::input_text(
            "Describe the issue:",
            CreateInputText::new(InputTextStyle::Paragraph, "ticket_body"),
        )),
    ]
}

pub struct TicketCreate;

#[async_trait]
impl ModuleComponent for TicketCreate {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("ticket_create"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        TicketComponent::ticket_create(&cx.ctx.http, cx.interaction, make_ticket_modal_components())
            .await
            .map_err(HandlerError::from_respond)
    }
}

pub struct SupportTicket;

#[async_trait]
impl ModuleComponent for SupportTicket {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("support_ticket"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        TicketComponent::ticket_create(&cx.ctx.http, cx.interaction, make_ticket_modal_components())
            .await
            .map_err(HandlerError::from_respond)
    }
}

pub struct SupportClose;

#[async_trait]
impl ModuleComponent for SupportClose {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("support_close"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        TicketComponent::support_close(&cx.ctx.http, cx.interaction)
            .await
            .map_err(HandlerError::from_respond)
    }
}

pub struct SupportFaq;

#[async_trait]
impl ModuleComponent for SupportFaq {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("support_faq"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        TicketComponent::support_faq::<Postgres, GuildTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
        )
        .await
        .map_err(HandlerError::from_respond)
    }
}

pub struct CreateTicketModal;

#[async_trait]
impl ModuleModal for CreateTicketModal {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("create_ticket"))
    }

    async fn run(&self, cx: &ModalCtx<'_>) -> Result<(), HandlerError> {
        TicketModal::run::<Postgres, GuildTable, TicketTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
        )
        .await
        .map_err(HandlerError::from_respond)
    }
}
