use std::collections::{HashMap, HashSet};

use crate::dto::destiny2::{CatalogWeaponInfo, LoadoutCatalog};
use crate::dto::destiny2_keys::{display_name, emoji_url, enum_key};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Super,
    ClassAbility,
    Jump,
    Melee,
    Grenade,
    Aspect,
    Fragment,
    WeaponPerk,
    ArmourMod,
    ArtifactPerk,
    Weapon,
    Armour,
    Artifact,
}

impl Field {
    #[must_use]
    pub const fn usage_key(self) -> &'static str {
        match self {
            Self::Super => "super",
            Self::ClassAbility => "class_ability",
            Self::Jump => "jump",
            Self::Melee => "melee",
            Self::Grenade => "grenade",
            Self::Aspect => "aspect",
            Self::Fragment => "fragment",
            Self::WeaponPerk => "weapon_perk",
            Self::ArmourMod => "armour_mod",
            Self::ArtifactPerk => "artifact_perk",
            Self::Weapon => "weapon",
            Self::Armour => "armour",
            Self::Artifact => "artifact",
        }
    }

    #[must_use]
    pub const fn allows_repeats(self) -> bool {
        matches!(self, Self::ArmourMod)
    }

    #[must_use]
    pub const fn is_emoji(self) -> bool {
        !matches!(self, Self::Weapon | Self::Armour | Self::Artifact)
    }

    #[must_use]
    pub const fn noun(self) -> &'static str {
        match self {
            Self::Super => "super",
            Self::ClassAbility => "class ability",
            Self::Jump => "jump",
            Self::Melee => "melee",
            Self::Grenade => "grenade",
            Self::Aspect => "aspect",
            Self::Fragment => "fragment",
            Self::WeaponPerk => "perk",
            Self::ArmourMod => "mod",
            Self::ArtifactPerk => "artifact perk",
            Self::Weapon => "weapon",
            Self::Armour => "armour piece",
            Self::Artifact => "artifact",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Candidate {
    pub key: String,
    pub label: String,
    pub detail: String,
    pub image: Option<String>,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Section {
    pub title: String,
    pub items: Vec<Candidate>,
}

#[derive(Clone, Copy, Debug)]
pub struct Scope<'a> {
    pub field: Field,
    pub class: &'a str,
    pub element: &'a str,
    pub weapon: Option<&'a str>,
    pub armour_slot: Option<&'a str>,
    pub taken: &'a [String],
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CatalogIndex {
    images: HashMap<String, String>,
    reserved: HashSet<String>,
}

impl CatalogIndex {
    #[must_use]
    pub fn new(c: &LoadoutCatalog) -> Self {
        let o = &c.options;
        let reserved = [
            &o.classes,
            &o.elements,
            &o.modes,
            &o.affinities,
            &o.archetypes,
            &o.stats,
            &o.armour_slots,
        ]
        .into_iter()
        .flatten()
        .map(|label| enum_key(label))
        .collect();
        let images =
            c.emojis.iter().map(|e| (e.name.clone(), emoji_url(&e.id))).collect();
        Self { images, reserved }
    }

    #[must_use]
    pub fn image(&self, key: &str) -> Option<String> {
        self.images.get(key).cloned()
    }

    #[must_use]
    pub fn has_emoji(&self, key: &str) -> bool {
        self.images.contains_key(key)
    }

    #[must_use]
    pub fn has_any_emoji(&self) -> bool {
        !self.images.is_empty()
    }

    #[must_use]
    pub fn is_reserved(&self, key: &str) -> bool {
        self.reserved.contains(key)
    }

    #[must_use]
    pub fn enum_image(&self, label: &str) -> Option<String> {
        self.image(&enum_key(label))
    }
}

#[must_use]
pub fn matches(query: &str, label: &str, key: &str) -> bool {
    let haystack =
        format!("{} {}", label.to_lowercase(), key.to_lowercase().replace('_', " "));
    query.to_lowercase().split_whitespace().all(|word| haystack.contains(word))
}

struct Builder<'a> {
    query: &'a str,
    taken: &'a [String],
    seen: HashSet<String>,
    sections: Vec<Section>,
}

impl Builder<'_> {
    fn push(&mut self, title: String, items: impl IntoIterator<Item = Candidate>) {
        let items: Vec<Candidate> = items
            .into_iter()
            .filter(|c| matches(self.query, &c.label, &c.key))
            .filter(|c| self.seen.insert(c.key.clone()))
            .map(|c| Candidate { selected: self.taken.contains(&c.key), ..c })
            .collect();
        if !items.is_empty() {
            self.sections.push(Section { title, items });
        }
    }
}

fn emoji_candidate(idx: &CatalogIndex, key: &str) -> Candidate {
    Candidate {
        key: key.to_owned(),
        label: display_name(key),
        detail: key.to_owned(),
        image: idx.image(key),
        selected: false,
    }
}

fn weapon_candidate(w: &CatalogWeaponInfo) -> Candidate {
    Candidate {
        key: w.name.clone(),
        label: w.name.clone(),
        detail: format!("{} {}", w.affinity, w.archetype),
        image: (!w.icon_url.is_empty()).then(|| w.icon_url.clone()),
        selected: false,
    }
}

fn used_keys(c: &LoadoutCatalog, scope: &Scope<'_>) -> (Vec<String>, Vec<String>) {
    let mut here: Vec<(u32, &str)> = Vec::new();
    let mut elsewhere: HashMap<&str, u32> = HashMap::new();
    for u in c.usage.iter().filter(|u| u.field == scope.field.usage_key()) {
        if u.class == scope.class && u.element == scope.element {
            here.push((u.uses, &u.key));
        } else {
            *elsewhere.entry(&u.key).or_default() += u.uses;
        }
    }
    let mut elsewhere: Vec<(u32, &str)> =
        elsewhere.into_iter().map(|(k, n)| (n, k)).collect();
    let order = |a: &(u32, &str), b: &(u32, &str)| b.0.cmp(&a.0).then(a.1.cmp(b.1));
    here.sort_by(order);
    elsewhere.sort_by(order);
    let keys =
        |v: Vec<(u32, &str)>| v.into_iter().map(|(_, k)| k.to_owned()).collect();
    (keys(here), keys(elsewhere))
}

#[must_use]
pub fn sections(
    c: &LoadoutCatalog,
    idx: &CatalogIndex,
    scope: &Scope<'_>,
    query: &str,
) -> Vec<Section> {
    let mut b = Builder {
        query,
        taken: scope.taken,
        seen: HashSet::new(),
        sections: Vec::new(),
    };
    let here_title = format!("Used in {} {} builds", scope.element, scope.class);

    match scope.field {
        Field::Weapon => {
            let (here, _) = used_keys(c, scope);
            let by_name = |name: &String| c.weapons.iter().find(|w| &w.name == name);
            b.push(
                here_title,
                here.iter().filter_map(by_name).map(weapon_candidate),
            );
            b.push("All weapons".to_owned(), c.weapons.iter().map(weapon_candidate));
        },
        Field::Artifact => {
            let (here, elsewhere) = used_keys(c, scope);
            let named = |name: &String| Candidate {
                key: name.clone(),
                label: name.clone(),
                detail: String::new(),
                image: None,
                selected: false,
            };
            b.push(here_title, here.iter().map(named));
            b.push("Used elsewhere".to_owned(), elsewhere.iter().map(named));
        },
        Field::Armour => {
            let slot = scope.armour_slot.unwrap_or_default();
            let pieces = c.armour.iter().filter(|a| a.slot == slot);
            let candidate = |a: &crate::dto::destiny2::ArmourPieceInfo| Candidate {
                key: a.name.clone(),
                label: a.name.clone(),
                detail: format!("{} {}", a.class, a.slot),
                image: (!a.icon_url.is_empty()).then(|| a.icon_url.clone()),
                selected: false,
            };
            let (own, other): (Vec<_>, Vec<_>) =
                pieces.partition(|a| a.class == scope.class);
            b.push(
                format!("Used in {} builds", scope.class),
                own.into_iter().map(candidate),
            );
            b.push("Other classes".to_owned(), other.into_iter().map(candidate));
        },
        field @ (Field::Super
        | Field::ClassAbility
        | Field::Jump
        | Field::Melee
        | Field::Grenade
        | Field::Aspect
        | Field::Fragment
        | Field::WeaponPerk
        | Field::ArmourMod
        | Field::ArtifactPerk) => {
            if field == Field::WeaponPerk
                && let Some(name) = scope.weapon.filter(|n| !n.is_empty())
                && let Some(w) = c.weapons.iter().find(|w| w.name == name)
            {
                b.push(
                    format!("Known perks for {name}"),
                    w.known_perks.iter().map(|k| emoji_candidate(idx, k)),
                );
            }
            let (here, elsewhere) = used_keys(c, scope);
            b.push(here_title, here.iter().map(|k| emoji_candidate(idx, k)));
            b.push(
                "Used elsewhere".to_owned(),
                elsewhere.iter().map(|k| emoji_candidate(idx, k)),
            );

            let mut other: Vec<&String> = idx.images.keys().collect();
            if field == Field::WeaponPerk {
                other.extend(&c.perks);
            }
            other.sort();
            other.dedup();
            b.push(
                "Other Zayden emojis".to_owned(),
                other
                    .into_iter()
                    .filter(|k| !idx.is_reserved(k))
                    .map(|k| emoji_candidate(idx, k)),
            );
        },
    }

    b.sections
}
