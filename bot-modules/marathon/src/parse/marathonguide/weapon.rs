use scraper::{ElementRef, Selector};

use super::{absolute, select_attr, select_text};
use crate::model::{AttachmentSlot, Stat, Weapon};
use crate::parse::html;
use crate::parse::lexical::{leading_number, slugify};

#[must_use]
pub fn marathonguide_html_to_weapon(slug: &str, page: &str) -> Option<Weapon> {
    let doc = html::document(page);
    let cards = Selector::parse("app-weapon").ok()?;
    doc.select(&cards).find_map(|card| card_to_weapon(slug, card))
}

fn card_to_weapon(slug: &str, card: ElementRef<'_>) -> Option<Weapon> {
    let name = select_text(card, "div.ml-3 div.text-lg")?;
    if slugify(&name) != slug {
        return None;
    }

    let stats = stats(card);
    let scalar = |label: &str| {
        stats.iter().find(|s| s.name == label).map(|s| s.value.as_str())
    };

    Some(Weapon {
        slug: slug.to_string(),
        name,
        weapon_type: select_text(card, "div.ml-3 div.opacity-75"),
        ammo_type: select_attr(card, "app-item img", "alt"),
        damage: None,
        fire_rate: None,
        magazine_size: scalar("Mag").and_then(leading_number),
        reload_speed: None,
        range: scalar("Range").and_then(leading_number),
        description: select_text(card, "div[class*=\"min-h-14\"]"),
        thumbnail_url: select_attr(card, "img.w-24", "src").as_deref().map(absolute),
        stats,
        attachment_slots: slots(card),
    })
}

fn stats(card: ElementRef<'_>) -> Vec<Stat> {
    let (Ok(cell), Ok(label), Ok(value)) = (
        Selector::parse("app-weapon-stats div.stat-cell"),
        Selector::parse("div.stat-label"),
        Selector::parse("div.stat-value"),
    ) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for c in card.select(&cell) {
        let (Some(l), Some(v)) = (c.select(&label).next(), c.select(&value).next())
        else {
            continue;
        };
        let name = html::element_text(l);
        let value = html::element_text(v);
        if !name.is_empty() && !value.is_empty() {
            out.push(Stat { name, value });
        }
    }
    out
}

fn slots(card: ElementRef<'_>) -> Vec<AttachmentSlot> {
    let Ok(sel) = Selector::parse(r#"img[alt$=" slot"]"#) else {
        return Vec::new();
    };
    card.select(&sel)
        .filter_map(|img| {
            let alt = img.value().attr("alt")?;
            let slot = alt.strip_suffix(" slot").unwrap_or(alt).trim();
            (!slot.is_empty())
                .then(|| AttachmentSlot { slot: slot.to_string(), attachment: None })
        })
        .collect()
}
