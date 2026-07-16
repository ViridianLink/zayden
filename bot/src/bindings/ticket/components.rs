use std::borrow::Cow;

use async_trait::async_trait;
use serenity::all::{
    CreateInputText,
    CreateLabel,
    CreateModalComponent,
    CreateSelectMenu,
    CreateSelectMenuKind,
    CreateSelectMenuOption,
    InputTextStyle,
};
use ticket::{TicketComponent, TicketModal, TicketStores};
use zayden_core::ctx::{ComponentCtx, ModalCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{ModuleComponent, ModuleModal};
use zayden_core::scope::IdMatch;

fn make_ticket_modal_components<'a>() -> Vec<CreateModalComponent<'a>> {
    vec![
        CreateModalComponent::Label(CreateLabel::select_menu(
            "Select a relevent category:",
            CreateSelectMenu::new("ticket_category", CreateSelectMenuKind::String {
                options: vec![
                    CreateSelectMenuOption::new("Complaint", "complaint"),
                    CreateSelectMenuOption::new(
                        "General question",
                        "general_question",
                    ),
                    CreateSelectMenuOption::new(
                        "Role Application",
                        "role_application",
                    ),
                    CreateSelectMenuOption::new("Suggestion", "suggestion"),
                ]
                .into(),
            }),
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
        TicketComponent::ticket_create(
            &cx.ctx.http,
            cx.interaction,
            make_ticket_modal_components(),
        )
        .await?;
        Ok(())
    }
}

pub struct SupportTicket;

#[async_trait]
impl ModuleComponent for SupportTicket {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("support_ticket"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        TicketComponent::ticket_create(
            &cx.ctx.http,
            cx.interaction,
            make_ticket_modal_components(),
        )
        .await?;
        Ok(())
    }
}

pub struct SupportClose;

#[async_trait]
impl ModuleComponent for SupportClose {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("support_close"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        TicketComponent::support_close(&cx.ctx.http, cx.interaction).await?;
        Ok(())
    }
}

pub struct SupportFaq;

#[async_trait]
impl ModuleComponent for SupportFaq {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("support_faq"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let stores = TicketStores {
            support: &cx.app.settings.support,
            ticket: &cx.app.settings.ticket,
        };

        TicketComponent::support_faq(
            &cx.ctx.http,
            cx.interaction,
            stores,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}

pub struct CreateTicketModal;

#[async_trait]
impl ModuleModal for CreateTicketModal {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("create_ticket"))
    }

    async fn run(&self, cx: &ModalCtx<'_>) -> Result<(), HandlerError> {
        let stores = TicketStores {
            support: &cx.app.settings.support,
            ticket: &cx.app.settings.ticket,
        };

        TicketModal::run(&cx.ctx.http, cx.interaction, stores, &cx.app.db).await?;
        Ok(())
    }
}
