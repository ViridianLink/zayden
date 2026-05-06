use destiny2_core::BungieClientData;
use futures::future;
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateAttachment, CreateCommand,
    CreateCommandOption, EditInteractionResponse, ResolvedOption, ResolvedValue,
};
use tokio::sync::RwLock;

use crate::endgame_analysis::{EndgameAnalysisSheet, Weapon};

pub struct DimWishlistCommand;

impl DimWishlistCommand {
    pub async fn run<Data: BungieClientData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        mut options: Vec<ResolvedOption<'_>>,
    ) {
        interaction.defer_ephemeral(&ctx.http).await.unwrap();

        let strict = match options.pop().map(|o| o.value) {
            Some(ResolvedValue::String(strict)) => strict,
            _ => "soft",
        };

        let tier = match strict {
            "soft" => vec!["S", "A", "B", "C", "D", "E", "F", "G"],
            "regular" => vec!["S", "A", "B", "C", "D"],
            "semi-strict" => vec!["S", "A", "B", "C"],
            "strict" => vec!["S", "A", "B"],
            "very strict" => vec!["S", "A"],
            "uber strict" => vec!["S"],
            _ => unreachable!(),
        };

        let (item_manifest, perk_manifest) = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            let client = data.bungie_client();
            let manifest = client.destiny_manifest().await.unwrap();
            future::try_join(
                client.destiny_inventory_item_definition(&manifest, "en"),
                client.destiny_plug_set_definition(&manifest, "en"),
            )
            .await
        }
        .unwrap();

        let weapons = match std::fs::read_to_string("weapons.json") {
            Ok(weapons) => weapons,
            Err(_) => {
                EndgameAnalysisSheet::update(&item_manifest).await.unwrap();
                std::fs::read_to_string("weapons.json").unwrap()
            }
        };
        let weapons: Vec<Weapon> = serde_json::from_str(&weapons).unwrap();

        let wishlist = weapons
            .into_iter()
            .filter(|weapon| {
                tier.contains(&weapon.tier.tier().as_str()) || weapon.tier.tier() == "None"
            })
            .map(|weapon| weapon.as_wishlist(&item_manifest, &perk_manifest))
            .collect::<Vec<_>>();

        let wishlist = format!("title: DIM Wishlist\n\n{}", wishlist.join("\n\n"));

        let file = CreateAttachment::bytes(wishlist, format!("PVE Wishlist ({strict}).txt"));

        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .new_attachment(file)
                    .content(format!("PVE Wishlist ({strict}):")),
            )
            .await
            .unwrap();
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("dimwishlist")
            .description("Get a wishlist from DIM")
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "strict", "Soft: All | Regular: S, A, B, C, D | Semi: S, A, B, C | Strict: S, A, B | Very: S, A | Uber: S")
                    .add_string_choice("Soft", "soft")
                    .add_string_choice("Regular", "regular")
                    .add_string_choice("Semi-strict", "semi-strict")
                    .add_string_choice("Strict", "strict")
                    .add_string_choice("Very Strict", "very strict")
                    .add_string_choice("Uber Strict", "uber strict"),
            )
    }
}
