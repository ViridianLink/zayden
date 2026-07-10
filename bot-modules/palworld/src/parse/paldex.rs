use crate::model::{
    ActiveSkill,
    Aura,
    Element,
    Item,
    Pal,
    PassiveSkill,
    Stats,
    Suitability,
};
use crate::transport::{RawItem, RawPal, RawPassive};

fn nonempty(value: Option<String>) -> Option<String> {
    value.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

#[must_use]
pub fn pal_from_raw(raw: RawPal) -> Pal {
    let elements =
        raw.types.iter().filter_map(|t| Element::parse(&t.name)).collect::<Vec<_>>();

    let stats = raw.stats.map(|s| Stats {
        hp: s.hp,
        attack_melee: s.attack.melee,
        attack_ranged: s.attack.ranged,
        defense: s.defense,
        speed_ride: s.speed.ride,
        speed_run: s.speed.run,
        speed_walk: s.speed.walk,
        stamina: s.stamina,
        support: s.support,
        food: s.food,
    });

    let suitability = raw
        .suitability
        .into_iter()
        .filter(|s| !s.kind.trim().is_empty())
        .map(|s| Suitability { kind: s.kind, level: s.level })
        .collect();

    let partner_skill = raw.aura.and_then(|a| {
        (!a.name.trim().is_empty()).then(|| Aura {
            name: a.name,
            description: nonempty(a.description),
            tech: nonempty(a.tech),
        })
    });

    let active_skills = raw
        .skills
        .into_iter()
        .filter(|s| !s.name.trim().is_empty())
        .map(|s| ActiveSkill {
            name: s.name,
            level: s.level,
            element: nonempty(s.element),
            cooldown: s.cooldown,
            power: s.power,
            description: nonempty(s.description),
        })
        .collect();

    let breeding = raw.breeding.unwrap_or_default();

    Pal {
        key: raw.key,
        paldex_no: raw.id,
        name: raw.name,
        elements,
        stats,
        suitability,
        drops: raw.drops.into_iter().filter(|d| !d.trim().is_empty()).collect(),
        partner_skill,
        active_skills,
        description: nonempty(raw.description),
        genus: nonempty(raw.genus),
        rarity: raw.rarity,
        price: raw.price,
        size: nonempty(raw.size),
        breeding_rank: breeding.rank,
        breeding_order: breeding.order,
        child_eligible: breeding.child_eligible,
        male_probability: breeding.male_probability,
        image_url: nonempty(raw.image_wiki)
            .map(|u| u.trim_end_matches('/').to_string()),
        wiki_url: nonempty(raw.wiki),
    }
}

#[must_use]
pub fn item_from_raw(raw: RawItem) -> Item {
    Item {
        key: raw.key,
        name: raw.name,
        item_type: nonempty(raw.item_type),
        description: nonempty(raw.description),
        gold: raw.gold,
        weight: raw.weight,
        image_url: None,
    }
}

#[must_use]
pub fn passive_from_raw(key: String, raw: RawPassive) -> PassiveSkill {
    PassiveSkill {
        key,
        name: raw.name,
        positive: nonempty(raw.positive),
        negative: nonempty(raw.negative),
        tier: raw.tier,
    }
}
