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
use super::markdown::{emoji_section, with_emoji_row};
use super::mode::Mode;
use super::{DUPLICATE, budget, resolve_emoji};
use crate::Result;
use crate::endgame_analysis::sheet::Affinity;

pub(crate) const SUBCLASS_HEADING: &str = "### SUBCLASS\nSuper       Abilities                                       Aspects";
pub(crate) const GEAR_HEADING: &str = "### GEAR AND MODS";

const CHOICE_LABEL: usize = 100;

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
            .chars()
            .take(CHOICE_LABEL)
            .collect()
    }

    pub async fn into_component<Data: EmojiCacheData>(
        self,
        ctx: &Context,
        parent_token: &str,
    ) -> Result<CreateComponent<'static>> {
        let data_lock = ctx.data::<tokio::sync::RwLock<Data>>();

        let mut owned_cache = {
            let data = data_lock.read().await;
            (*data.emojis()).clone()
        };

        let component =
            resolve_emoji(&mut owned_cache, ctx, parent_token, |cache| {
                self.container(cache)
            })
            .await?;

        data_lock.write().await.emojis_mut().merge_from(&owned_cache);

        Ok(component)
    }

    pub fn container(
        &self,
        cache: &EmojiCache,
    ) -> EmojiResult<CreateComponent<'static>> {
        let text = |content: String| {
            CreateContainerComponent::TextDisplay(CreateTextDisplay::new(content))
        };
        let line_sep = || {
            CreateContainerComponent::Separator(CreateSeparator::new().divider(true))
        };

        let element_key = self.element.key();
        let subclass_btn = CreateButton::new(element_key.clone())
            .label(self.element.to_string())
            .emoji(cache.emoji(&element_key)?)
            .style(ButtonStyle::Secondary);
        let tag_buttons = iter::once(subclass_btn)
            .chain(iter::once(button(self.mode.to_string())))
            .chain(self.tags.iter().cloned().map(button))
            .collect::<Vec<_>>();
        let tags = CreateContainerComponent::ActionRow(CreateActionRow::buttons(
            tag_buttons,
        ));

        let mut details = format!("By {}", self.author);
        if let Some(url) = &self.video_url {
            let _ = write!(details, " • [Video Guide]({url})");
        }
        let heading = text(format!(
            "-# {element} {class} Build\n# {class}  •  {}  •  {}\n{details}",
            self.super_name,
            self.name,
            element = self.element,
            class = self.class,
        ));

        let dim_link =
            CreateContainerComponent::ActionRow(CreateActionRow::buttons(vec![
                CreateButton::new_link(self.dim_link.clone())
                    .label("COPY DIM LINK")
                    .emoji(DUPLICATE),
            ]));

        let aspects = self
            .aspects
            .iter()
            .map(|a| cache.emoji_str(&a.emoji))
            .collect::<EmojiResult<Vec<String>>>()?
            .join(" ");
        let fragments = self
            .aspects
            .iter()
            .flat_map(|a| &a.fragments)
            .map(|frag| Ok(format!(" {}", cache.emoji_str(frag)?)))
            .collect::<EmojiResult<String>>()?;
        let mut subclass = format!(
            "{SUBCLASS_HEADING}\n# {}    {} {} {} {}    {aspects}",
            cache.emoji_str(&self.super_emoji)?,
            cache.emoji_str(&self.class_ability)?,
            cache.emoji_str(&self.jump)?,
            cache.emoji_str(&self.melee)?,
            cache.emoji_str(&self.grenade)?,
        );
        if !fragments.is_empty() {
            let _ = write!(subclass, "\n\nFragments\n#{fragments}");
        }

        let weapons = self.weapon_components(cache)?;
        let armour = self.armour_components(cache)?;
        let stat_prio = self.stat_prio_str(cache)?;
        let artifact = self
            .artifact_perks
            .iter()
            .map(|p| cache.emoji_str(p))
            .collect::<EmojiResult<Vec<String>>>()?
            .join(" ");

        let misc_sections: Vec<String> = [
            emoji_section("### Stats Priority", &stat_prio),
            emoji_section("### ARTIFACT PERKS", &format!(" {artifact}")),
            self.how_it_works.as_ref().map(|h| format!("### HOW IT WORKS\n# {h}")),
        ]
        .into_iter()
        .flatten()
        .collect();
        let misc =
            (!misc_sections.is_empty()).then(|| text(misc_sections.join("\n")));

        let base = budget::base_components(
            self.tags.len(),
            weapons.len(),
            armour.len(),
            misc.is_some(),
        );
        let spacer = budget::has_spacer(weapons.len(), base).then(|| {
            CreateContainerComponent::Separator(
                CreateSeparator::new().spacing(SeparatorSpacingSize::Large),
            )
        });

        let mut components = Vec::with_capacity(base);
        components.extend([
            heading,
            tags,
            line_sep(),
            dim_link,
            line_sep(),
            text(subclass),
            line_sep(),
            text(GEAR_HEADING.to_owned()),
        ]);
        components.extend(weapons);
        components.extend(spacer);
        components.extend(armour);
        components.extend(misc);

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

                let text = CreateTextDisplay::new(with_emoji_row(
                    &format!(
                        "**{}**\n{affinity_emoji} {}",
                        weapon.name, weapon.archetype
                    ),
                    &perks,
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

                let content = with_emoji_row(&format!("**{}**", armour.name), &mods);

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
