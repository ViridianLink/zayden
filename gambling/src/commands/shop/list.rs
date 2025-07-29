use std::time::Duration;

use futures::StreamExt;
use serenity::all::{
    ButtonStyle, CollectComponentInteractions, CommandInteraction, Context, CreateActionRow,
    CreateButton, CreateComponent, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage, EditInteractionResponse, ResolvedOption, ResolvedValue,
};
use sqlx::{Database, Pool};
use zayden_core::FormatNum;

use crate::{
    COIN, Coins, ItemInventory, Result, SHOP_ITEMS, ShopPage,
    commands::shop::{BuyRow, ShopManager},
    shop::SALES_TAX,
};

pub async fn list<Db: Database, Manager: ShopManager<Db>>(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &Pool<Db>,
    options: &[ResolvedOption<'_>],
) -> Result<()> {
    let page = match options.first().map(|opt| &opt.value) {
        Some(ResolvedValue::String(page)) => page.parse().unwrap(),
        _ => ShopPage::Item,
    };

    let row = match Manager::buy_row(pool, interaction.user.id).await.unwrap() {
        Some(row) => row,
        None => BuyRow::new(interaction.user.id),
    };

    let embed = create_embed(page, &row);

    let prev = CreateButton::new("shop_prev")
        .label("<")
        .style(ButtonStyle::Secondary);
    let next = CreateButton::new("shop_next")
        .label(">")
        .style(ButtonStyle::Secondary);

    let msg = interaction
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().embed(embed).components(vec![
                CreateComponent::ActionRow(CreateActionRow::buttons(vec![prev, next])),
            ]),
        )
        .await?;

    let mut stream = msg
        .id
        .collect_component_interactions(ctx)
        .timeout(Duration::from_secs(120))
        .stream();

    while let Some(interaction) = stream.next().await {
        let title = interaction
            .message
            .embeds
            .first()
            .and_then(|embed| embed.title.as_deref());

        let (embed, components) = if interaction.data.custom_id == "shop_next" {
            shop(&row, title, 1)
        } else {
            shop(&row, title, -1)
        };

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .components(vec![components]),
                ),
            )
            .await?
    }

    interaction
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().components(Vec::new()),
        )
        .await?;

    Ok(())
}

fn shop<'a>(
    row: &'a BuyRow,
    title: Option<&str>,
    page_change: i8,
) -> (CreateEmbed<'a>, CreateComponent<'a>) {
    let current_cat = title
        .map(|title| title.strip_suffix(" Shop").unwrap().parse().unwrap())
        .unwrap_or(ShopPage::Item);

    let category_idx = ShopPage::pages()
        .iter()
        .position(|cat| *cat == current_cat)
        .unwrap() as i8;

    let category = ShopPage::pages()
        .get(usize::try_from(category_idx + page_change).unwrap_or_default())
        .copied()
        .unwrap_or(ShopPage::Item);

    let embed = create_embed(category, row);

    let prev = CreateButton::new("shop_prev")
        .label("<")
        .style(ButtonStyle::Secondary);
    let next = CreateButton::new("shop_next")
        .label(">")
        .style(ButtonStyle::Secondary);

    (
        embed,
        CreateComponent::ActionRow(CreateActionRow::buttons(vec![prev, next])),
    )
}

fn create_embed(category: ShopPage, row: &BuyRow) -> CreateEmbed<'_> {
    let inv = row.inventory();

    let items = SHOP_ITEMS
        .iter()
        .filter(|item| item.category == category)
        .map(|item| {
            let costs = item
                .costs(1)
                .into_iter()
                .map(|(cost, currency)| format!("`{}` {}", cost.format(), currency))
                .collect::<Vec<_>>();

            let mut s = format!("**{item}**");

            if !item.description.is_empty() {
                s.push('\n');
                s.push_str(item.description);
            }

            s.push_str(&format!(
                "\nOwned: `{}`\nCost:",
                inv.iter()
                    .find(|inv_item| inv_item.item_id == item.id)
                    .map(|item| item.quantity)
                    .unwrap_or_default()
            ));

            if costs.len() == 1 {
                s.push(' ');
                s.push_str(&costs.join(""));
            } else {
                s.push('\n');
                s.push_str(&costs.join("\n"));
            }

            s
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let desc = format!(
        "Sales tax: {}%\nYour coins: {}  <:coin:{COIN}>\n--------------------\n{items}\n--------------------\nBuy with `/shop buy <item> <amount>`\nSell with `/shop sell <item> <amount>`",
        SALES_TAX * 100.0,
        row.coins_str()
    );

    CreateEmbed::new()
        .title(format!("{category} Shop"))
        .description(desc)
}
