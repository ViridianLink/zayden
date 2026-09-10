use std::borrow::Cow;
use std::fmt::Write as _;

use serenity::all::{
    ButtonStyle,
    CreateActionRow,
    CreateButton,
    CreateComponent,
    CreateInputText,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    CreateLabel,
    CreateModal,
    CreateModalComponent,
    CreateSelectMenu,
    CreateSelectMenuKind,
    CreateSelectMenuOption,
    CreateTextDisplay,
    InputTextStyle,
};
use zayden_app::config::HostingGame;
use zayden_app::entitlement::Tier;

use crate::pricing::{self, Plan};

pub const MODAL_PREFIX: &str = "hosting_spec:";
pub const CONFIRM_PREFIX: &str = "hosting_confirm:";
pub const CANCEL_PREFIX: &str = "hosting_cancel:";

pub const PLAN_FIELD: &str = "hosting_plan";
pub const NAME_FIELD: &str = "hosting_name";

pub fn spec_modal(game: &HostingGame, tier: Tier) -> CreateModal<'_> {
    let components = vec![
        CreateModalComponent::TextDisplay(CreateTextDisplay::new(rate_card(
            game, tier,
        ))),
        CreateModalComponent::Label(CreateLabel::select_menu(
            "Size",
            CreateSelectMenu::new(PLAN_FIELD, CreateSelectMenuKind::String {
                options: plan_options(tier).into(),
            }),
        )),
        CreateModalComponent::Label(CreateLabel::input_text(
            "Server name",
            CreateInputText::new(InputTextStyle::Short, NAME_FIELD)
                .placeholder(&game.name)
                .max_length(48)
                .required(false),
        )),
    ];

    CreateModal::new(
        format!("{MODAL_PREFIX}{}", game.key),
        format!("Host {}", truncate(&game.name, 32)),
    )
    .components(components)
}

fn plan_options<'a>(tier: Tier) -> Vec<CreateSelectMenuOption<'a>> {
    Plan::ALL
        .into_iter()
        .map(|plan| {
            let price = pricing::price_cents(plan, tier);
            let suffix = if price == 0 {
                "included".to_owned()
            } else {
                format!("+{}", pricing::format_price(price))
            };

            CreateSelectMenuOption::new(
                format!("{} ({suffix})", plan.label()),
                plan.key(),
            )
            .description(format!(
                "{} GB RAM · {}% CPU · {} GB disk",
                plan.memory_mib() / 1024,
                plan.cpu_percent(),
                plan.disk_mib() / 1024
            ))
        })
        .collect()
}

fn rate_card(game: &HostingGame, tier: Tier) -> String {
    let mut card = format!("**{}** — monthly, cancel any time\n", game.name);

    for plan in Plan::ALL {
        let price = pricing::price_cents(plan, tier);
        let _ = writeln!(
            card,
            "· **{}** {} GB · {}% CPU · {} GB — {}",
            plan.label(),
            plan.memory_mib() / 1024,
            plan.cpu_percent(),
            plan.disk_mib() / 1024,
            pricing::format_price(price)
        );
    }

    if tier == Tier::Free {
        card.push_str("\nPro and Ultra subscribers pay less, or nothing at all.");
    } else {
        let _ =
            write!(card, "\nPrices shown include your {} discount.", tier.as_str());
    }

    card
}

#[must_use]
pub fn confirm_response<'a>(
    row_id: i64,
    summary: &str,
) -> CreateInteractionResponse<'a> {
    let buttons = vec![
        CreateButton::new(format!("{CONFIRM_PREFIX}{row_id}"))
            .style(ButtonStyle::Success)
            .label("Create it"),
        CreateButton::new(format!("{CANCEL_PREFIX}{row_id}"))
            .style(ButtonStyle::Secondary)
            .label("Cancel"),
    ];

    CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(summary.to_owned())
            .ephemeral(true)
            .components(vec![CreateComponent::ActionRow(CreateActionRow::buttons(
                buttons,
            ))]),
    )
}

#[must_use]
pub fn truncate(s: &str, max: usize) -> Cow<'_, str> {
    if s.chars().count() <= max {
        Cow::Borrowed(s)
    } else {
        Cow::Owned(s.chars().take(max).collect())
    }
}
