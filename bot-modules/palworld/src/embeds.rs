use std::fmt::Write as _;

use serenity::all::{
    Colour,
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
};

use crate::model::{Element, Item, Pal, PassiveSkill};

const ACCENT: Colour = Colour::from_rgb(0x35, 0xc7, 0x59);

fn separator() -> CreateContainerComponent<'static> {
    CreateContainerComponent::Separator(CreateSeparator::new().divider(true))
}

fn text(content: impl Into<String>) -> CreateContainerComponent<'static> {
    CreateContainerComponent::TextDisplay(CreateTextDisplay::new(content.into()))
}

fn body_component(
    content: String,
    thumbnail_url: Option<&str>,
) -> CreateContainerComponent<'static> {
    match thumbnail_url {
        Some(url) => CreateContainerComponent::Section(CreateSection::new(
            vec![CreateSectionComponent::TextDisplay(CreateTextDisplay::new(
                content,
            ))],
            CreateSectionAccessory::Thumbnail(CreateThumbnail::new(
                CreateUnfurledMediaItem::new(url.to_string()),
            )),
        )),
        None => text(content),
    }
}

fn labelled_list(
    title: &str,
    lines: &[String],
) -> CreateContainerComponent<'static> {
    if lines.is_empty() {
        return text(format!("### {title}\n*Data unavailable.*"));
    }
    text(format!("### {title}\n{}", lines.join("\n")))
}

fn container(
    components: Vec<CreateContainerComponent<'static>>,
) -> CreateComponent<'static> {
    CreateComponent::Container(
        CreateContainer::new(components).accent_colour(ACCENT),
    )
}

fn elements_line(elements: &[Element]) -> Option<String> {
    (!elements.is_empty())
        .then(|| elements.iter().map(|e| e.label()).collect::<Vec<_>>().join(" / "))
}

pub fn pal_component(pal: &Pal) -> CreateComponent<'static> {
    let mut body = format!("# {}", pal.name);

    let mut subtitle: Vec<String> = Vec::new();
    if pal.paldex_no > 0 {
        subtitle.push(format!("#{:03}", pal.paldex_no));
    }
    if let Some(elements) = elements_line(&pal.elements) {
        subtitle.push(elements);
    }
    if let Some(genus) = &pal.genus {
        subtitle.push(genus.clone());
    }
    if let Some(rarity) = pal.rarity {
        subtitle.push(format!("Rarity {rarity}"));
    }
    if !subtitle.is_empty() {
        let _ = write!(body, "\n-# {}", subtitle.join(" • "));
    }

    if let Some(stats) = &pal.stats {
        let _ = write!(
            body,
            "\n**HP:** {} • **Atk (M/R):** {}/{} • **Def:** {}",
            stats.hp, stats.attack_melee, stats.attack_ranged, stats.defense
        );
    }

    if let Some(rank) = pal.breeding_rank {
        let _ = write!(body, "\n**Breeding rank:** {rank}");
    }

    if let Some(description) = &pal.description {
        let _ = write!(body, "\n\n{description}");
    }

    let mut components =
        vec![body_component(body, pal.image_url.as_deref()), separator()];

    let work: Vec<String> = pal
        .suitability
        .iter()
        .map(|s| format!("**{}** Lv {}", s.kind.replace('_', " "), s.level))
        .collect();
    components.push(labelled_list("Work Suitability", &work));

    let drops: Vec<String> =
        pal.drops.iter().map(|d| format!("- {}", d.replace('_', " "))).collect();
    components.push(labelled_list("Drops", &drops));

    if let Some(aura) = &pal.partner_skill {
        let mut line =
            format!("### Partner Skill\n**{}**", aura.name.replace('_', " "));
        if let Some(desc) = &aura.description {
            let _ = write!(line, "\n{desc}");
        }
        components.push(text(line));
    }

    if !pal.active_skills.is_empty() {
        let lines: Vec<String> = pal
            .active_skills
            .iter()
            .map(|s| {
                let power =
                    s.power.map_or_else(String::new, |p| format!(" • Pwr {p}"));
                format!("**{}** (Lv {}){}", s.name.replace('_', " "), s.level, power)
            })
            .collect();
        components.push(labelled_list("Active Skills", &lines));
    }

    container(components)
}

pub fn breeding_component(
    a: &Pal,
    b: &Pal,
    child: &Pal,
    unique: bool,
) -> CreateComponent<'static> {
    let mut body = format!(
        "# Breeding Result\n**{}** × **{}** → **{}**",
        a.name, b.name, child.name
    );
    if unique {
        let _ = write!(body, "\n-# ✨ Special combination");
    }
    if let Some(elements) = elements_line(&child.elements) {
        let _ = write!(body, "\n**Element:** {elements}");
    }
    if let Some(rank) = child.breeding_rank {
        let _ = write!(body, "\n**Breeding rank:** {rank}");
    }

    container(vec![body_component(body, child.image_url.as_deref())])
}

pub fn breed_for_component(
    target: &Pal,
    pairs: &[(String, String)],
    total: usize,
) -> CreateComponent<'static> {
    let body = format!(
        "# Breeding Combinations\nParent pairs that produce **{}**",
        target.name
    );

    let mut components =
        vec![body_component(body, target.image_url.as_deref()), separator()];

    if pairs.is_empty() {
        components.push(text(
            "*No known breeding combinations — this Pal may be catch-only.*",
        ));
    } else {
        let lines: Vec<String> =
            pairs.iter().map(|(a, b)| format!("**{a}** × **{b}**")).collect();
        components.push(labelled_list("Combinations", &lines));
        if total > pairs.len() {
            components.push(text(format!(
                "-# Showing {} of {} combinations.",
                pairs.len(),
                total
            )));
        }
    }

    container(components)
}

pub fn item_component(item: &Item) -> CreateComponent<'static> {
    let mut body = format!("# {}", item.name);
    if let Some(item_type) = &item.item_type {
        let _ = write!(body, "\n-# {}", item_type.replace('_', " "));
    }
    if let Some(gold) = item.gold {
        let _ = write!(body, "\n**Sell price:** {gold} gold");
    }
    if let Some(weight) = item.weight {
        let _ = write!(body, "\n**Weight:** {weight}");
    }
    if let Some(description) = &item.description {
        let _ = write!(body, "\n\n{description}");
    }

    container(vec![body_component(body, item.image_url.as_deref())])
}

pub fn passive_component(skill: &PassiveSkill) -> CreateComponent<'static> {
    let mut body = format!("# {}", skill.name);
    let _ = write!(body, "\n-# Tier {}", skill.tier);
    if let Some(positive) = &skill.positive {
        let _ = write!(body, "\n**Effect:** {positive}");
    }
    if let Some(negative) = &skill.negative {
        let _ = write!(body, "\n**Drawback:** {negative}");
    }

    container(vec![text(body)])
}

pub fn type_component(
    element: Element,
    strong: &[Element],
    weak: &[Element],
    pals: &[String],
) -> CreateComponent<'static> {
    let body = format!("# {} Type", element.label());

    let join = |els: &[Element]| -> String {
        if els.is_empty() {
            "—".to_string()
        } else {
            els.iter().map(|e| e.label()).collect::<Vec<_>>().join(", ")
        }
    };

    let effectiveness =
        format!("**Strong against:** {}\n**Weak to:** {}", join(strong), join(weak));

    let mut components = vec![text(body), text(effectiveness), separator()];

    let list: Vec<String> = pals.iter().map(|p| format!("- {p}")).collect();
    components.push(labelled_list(&format!("{} Pals", element.label()), &list));

    container(components)
}
