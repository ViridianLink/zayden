use std::fmt::Write as _;
use std::iter;

use serenity::all::{
    ButtonStyle,
    Context,
    CreateActionRow,
    CreateButton,
    CreateComponent,
    CreateContainer,
    CreateContainerComponent,
    CreateSection,
    CreateSectionAccessory,
    CreateSectionComponent,
    CreateSeparator,
    CreateTextDisplay,
    CreateThumbnail,
    CreateUnfurledMediaItem,
    SeparatorSpacingSize,
};
use zayden_core::{EmojiCache, EmojiCacheData, EmojiResult};

use super::domain::{Archetype, ArmourSlot, Class, Element, StatKind};
use super::mode::Mode;
use super::{DUPLICATE, resolve_emoji};
use crate::Result;
use crate::endgame_analysis::sheet::Affinity;

#[derive(Debug, Clone)]
pub struct AspectRecord {
    pub emoji: String,
    pub fragments: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WeaponRecord {
    pub name: String,
    pub affinity: Affinity,
    pub archetype: Archetype,
    pub icon_url: String,
    pub perks: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ArmourRecord {
    pub slot: ArmourSlot,
    pub name: String,
    pub icon_url: String,
    pub mods: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LoadoutRecord {
    pub id: i32,
    pub name: String,
    pub class: Class,
    pub element: Element,
    pub mode: Mode,
    pub tags: Vec<String>,
    pub super_name: String,
    pub super_emoji: String,
    pub class_ability: String,
    pub jump: String,
    pub melee: String,
    pub grenade: String,
    pub aspects: Vec<AspectRecord>,
    pub weapons: Vec<WeaponRecord>,
    pub armour: Vec<ArmourRecord>,
    pub stats: Vec<(StatKind, i16)>,
    pub artifact_name: Option<String>,
    pub artifact_perks: Vec<String>,
    pub author: String,
    pub dim_link: String,
    pub video_url: Option<String>,
    pub how_it_works: Option<String>,
}

impl LoadoutRecord {
    #[must_use]
    pub fn choice_value(&self) -> String {
        self.id.to_string()
    }

    #[must_use]
    pub fn choice_label(&self) -> String {
        format!("{} | {}", self.element, self.name)
    }

    #[expect(
        clippy::significant_drop_tightening,
        reason = "emoji_cache borrows from the write guard; dropping it early would dangle"
    )]
    pub async fn into_component<Data: EmojiCacheData>(
        self,
        ctx: &Context,
        parent_token: &str,
    ) -> Result<CreateComponent<'static>> {
        let data_lock = ctx.data::<tokio::sync::RwLock<Data>>();
        let mut data = data_lock.write().await;
        let emoji_cache = data.emojis_mut();

        let mut components = Vec::with_capacity(21);

        let subclass_btn = resolve_emoji(emoji_cache, ctx, parent_token, |cache| {
            let key = self.element.key();
            let emoji = cache.emoji(&key)?;
            Ok(CreateButton::new(key)
                .label(self.element.to_string())
                .emoji(emoji)
                .style(ButtonStyle::Secondary))
        })
        .await?;

        let tag_buttons = iter::once(subclass_btn)
            .chain(iter::once(button(self.mode.to_string())))
            .chain(self.tags.iter().cloned().map(button))
            .collect::<Vec<_>>();
        let tags = CreateContainerComponent::ActionRow(CreateActionRow::buttons(
            tag_buttons,
        ));

        let heading1 =
            CreateContainerComponent::TextDisplay(CreateTextDisplay::new(format!(
                "-# {} {} Build",
                self.element, self.class
            )));

        let mut details = format!("By {}", self.author);
        if let Some(url) = &self.video_url {
            let _ = write!(details, " • [Video Guide]({url})");
        }

        let heading2 =
            CreateContainerComponent::TextDisplay(CreateTextDisplay::new(format!(
                "# {}  •  {}  •  {}\n{details}",
                self.class, self.super_name, self.name
            )));

        let line_sep = CreateContainerComponent::Separator(
            CreateSeparator::new().divider(true),
        );

        let dim_link =
            CreateContainerComponent::ActionRow(CreateActionRow::buttons(vec![
                CreateButton::new_link(self.dim_link.clone())
                    .label("COPY DIM LINK")
                    .emoji(DUPLICATE),
            ]));

        let subclass_heading = CreateContainerComponent::TextDisplay(
            CreateTextDisplay::new(
                "### SUBCLASS\nSuper       Abilities                                       Aspects",
            ),
        );

        let aspects = resolve_emoji(emoji_cache, ctx, parent_token, |cache| {
            self.aspects
                .iter()
                .map(|a| cache.emoji_str(&a.emoji))
                .collect::<EmojiResult<Vec<String>>>()
                .map(|v| v.join(" "))
        })
        .await?;

        let super_emoji = resolve_emoji(emoji_cache, ctx, parent_token, |cache| {
            cache.emoji_str(&self.super_emoji)
        })
        .await?;

        let class_emoji = resolve_emoji(emoji_cache, ctx, parent_token, |cache| {
            cache.emoji_str(&self.class_ability)
        })
        .await?;

        let jump_emoji = resolve_emoji(emoji_cache, ctx, parent_token, |cache| {
            cache.emoji_str(&self.jump)
        })
        .await?;

        let melee_emoji = resolve_emoji(emoji_cache, ctx, parent_token, |cache| {
            cache.emoji_str(&self.melee)
        })
        .await?;

        let grenade_emoji = resolve_emoji(emoji_cache, ctx, parent_token, |cache| {
            cache.emoji_str(&self.grenade)
        })
        .await?;

        let subclass = CreateContainerComponent::TextDisplay(
            CreateTextDisplay::new(format!(
                "# {super_emoji}    {class_emoji} {jump_emoji} {melee_emoji} {grenade_emoji}    {aspects}\n\nFragments",
            )),
        );

        let fragments_str = resolve_emoji(emoji_cache, ctx, parent_token, |cache| {
            self.aspects
                .iter()
                .flat_map(|a| &a.fragments)
                .map(|frag| {
                    let emoji = cache.emoji_str(frag)?;
                    Ok(format!(" {emoji}"))
                })
                .collect::<EmojiResult<String>>()
        })
        .await?;

        let fragments = CreateContainerComponent::TextDisplay(
            CreateTextDisplay::new(format!("#{fragments_str}")),
        );

        let gear_and_mods_heading = CreateContainerComponent::TextDisplay(
            CreateTextDisplay::new("### GEAR AND MODS"),
        );

        let weapons = resolve_emoji(emoji_cache, ctx, parent_token, |cache| {
            self.weapon_components(cache)
        })
        .await?;

        let armour = resolve_emoji(emoji_cache, ctx, parent_token, |cache| {
            self.armour_components(cache)
        })
        .await?;

        let stat_prio = resolve_emoji(emoji_cache, ctx, parent_token, |cache| {
            self.stat_prio_str(cache)
        })
        .await?;

        let artifact = resolve_emoji(emoji_cache, ctx, parent_token, |cache| {
            self.artifact_perks
                .iter()
                .map(|p| cache.emoji_str(p))
                .collect::<EmojiResult<Vec<String>>>()
                .map(|v| v.join(" "))
        })
        .await?;

        let mut misc_content = format!(
            "### Stats Priority\n#{stat_prio}\n### ARTIFACT PERKS\n# {artifact}",
        );

        if let Some(how_it_works) = &self.how_it_works {
            misc_content.push_str("\n### HOW IT WORKS\n# ");
            misc_content.push_str(how_it_works);
        }

        let misc = CreateContainerComponent::TextDisplay(CreateTextDisplay::new(
            misc_content,
        ));

        components.extend([
            heading1,
            heading2,
            tags,
            line_sep.clone(),
            dim_link,
            line_sep.clone(),
            subclass_heading,
            subclass,
            fragments,
            line_sep,
            gear_and_mods_heading,
        ]);
        if !weapons.is_empty() {
            components.extend(weapons);
            components.push(CreateContainerComponent::Separator(
                CreateSeparator::new().spacing(SeparatorSpacingSize::Large),
            ));
        }
        components.extend(armour);
        components.push(misc);

        Ok(CreateComponent::Container(CreateContainer::new(components)))
    }

    fn weapon_components(
        &self,
        emoji_cache: &EmojiCache,
    ) -> EmojiResult<Vec<CreateContainerComponent<'static>>> {
        self.weapons
            .iter()
            .map(|weapon| {
                let perks = weapon
                    .perks
                    .iter()
                    .map(|p| {
                        let emoji = emoji_cache.emoji_str(p)?;
                        Ok(format!(" {emoji}"))
                    })
                    .collect::<EmojiResult<String>>()?;

                let affinity_emoji = emoji_cache
                    .emoji_str(&weapon.affinity.to_string().to_lowercase())?;

                let text = CreateTextDisplay::new(format!(
                    "**{}**\n{affinity_emoji} {}\n#{perks}",
                    weapon.name, weapon.archetype,
                ));

                let thumbnail = CreateThumbnail::new(CreateUnfurledMediaItem::new(
                    weapon.icon_url.clone(),
                ));

                Ok(CreateContainerComponent::Section(CreateSection::new(
                    vec![CreateSectionComponent::TextDisplay(text)],
                    CreateSectionAccessory::Thumbnail(thumbnail),
                )))
            })
            .collect()
    }

    fn armour_components(
        &self,
        emoji_cache: &EmojiCache,
    ) -> EmojiResult<Vec<CreateContainerComponent<'static>>> {
        self.armour
            .iter()
            .map(|armour| {
                let mods = armour
                    .mods
                    .iter()
                    .map(|m| {
                        let emoji = emoji_cache.emoji_str(m)?;
                        Ok(format!(" {emoji}"))
                    })
                    .collect::<EmojiResult<String>>()?;

                let content = if mods.is_empty() {
                    format!("**{}**", armour.name)
                } else {
                    format!("**{}**\n#{mods}", armour.name)
                };

                let thumbnail = CreateThumbnail::new(CreateUnfurledMediaItem::new(
                    armour.icon_url.clone(),
                ));

                Ok(CreateContainerComponent::Section(CreateSection::new(
                    vec![CreateSectionComponent::TextDisplay(
                        CreateTextDisplay::new(content),
                    )],
                    CreateSectionAccessory::Thumbnail(thumbnail),
                )))
            })
            .collect()
    }

    fn stat_prio_str(&self, emoji_cache: &EmojiCache) -> EmojiResult<String> {
        self.stats
            .iter()
            .enumerate()
            .map(|(i, (stat, value))| {
                let emoji = emoji_cache.emoji_str(&stat.to_string())?;

                let s =
                    if *value < 200 { format!("`{value}` {emoji}") } else { emoji };

                let s = if i == 0 { format!(" {s}") } else { format!(" → {s}") };

                Ok(s)
            })
            .collect()
    }
}

fn button(label: String) -> CreateButton<'static> {
    CreateButton::new(label.clone()).label(label).style(ButtonStyle::Secondary)
}
