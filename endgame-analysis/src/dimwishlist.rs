use futures::{StreamExt, future, stream};
use serenity::all::{
    CommandInteraction, CommandOptionType, CreateAttachment, CreateCommand, CreateCommandOption,
    EditInteractionResponse, Http, ResolvedOption, ResolvedValue,
};
use sqlx::{Database, Pool};

use crate::{
    DestinyPerkManager, DestinyWeaponManager,
    endgame_analysis::{EndgameAnalysisSheet, Weapon},
};

pub struct DimWishlistCommand;

impl DimWishlistCommand {
    pub async fn run<
        Db: Database,
        WeaponManager: DestinyWeaponManager<Db>,
        PerkManager: DestinyPerkManager<Db>,
    >(
        http: &Http,
        interaction: &CommandInteraction,
        mut options: Vec<ResolvedOption<'_>>,
        pool: &Pool<Db>,
    ) {
        interaction.defer_ephemeral(http).await.unwrap();

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

        let weapons = match std::fs::read_to_string("weapons.json") {
            Ok(weapons) => weapons,
            Err(_) => {
                EndgameAnalysisSheet::update::<Db, WeaponManager>(pool)
                    .await
                    .unwrap();
                std::fs::read_to_string("weapons.json").unwrap()
            }
        };
        let weapons: Vec<Weapon> = serde_json::from_str(&weapons).unwrap();

        let wishlist = stream::iter(weapons)
            .filter(|weapon| future::ready(tier.contains(&weapon.tier.tier().as_str())))
            .then(|weapon| {
                let pool = pool.clone();
                async move {
                    weapon
                        .as_wishlist::<Db, WeaponManager, PerkManager>(&pool)
                        .await
                }
            })
            .collect::<Vec<_>>()
            .await;

        let wishlist = format!("title: DIM Wishlist\n\n{}", wishlist.join("\n\n"));

        let file = CreateAttachment::bytes(wishlist, format!("PVE Wishlist ({strict}).txt"));

        interaction
            .edit_response(
                http,
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
