use leptos::prelude::*;

use crate::dto::destiny2::{
    ArmourForm,
    AspectForm,
    LoadoutForm,
    StatForm,
    WeaponForm,
};

#[derive(Clone, Copy)]
pub struct AspectRow {
    pub key: u32,
    pub aspect: RwSignal<String>,
    pub fragments: RwSignal<Vec<String>>,
}

impl AspectRow {
    fn new(key: u32, a: AspectForm) -> Self {
        Self {
            key,
            aspect: RwSignal::new(a.aspect),
            fragments: RwSignal::new(a.fragments),
        }
    }

    fn to_form(self) -> AspectForm {
        AspectForm {
            aspect: self.aspect.get_untracked(),
            fragments: self.fragments.get_untracked(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct WeaponRow {
    pub key: u32,
    pub name: RwSignal<String>,
    pub affinity: RwSignal<String>,
    pub archetype: RwSignal<String>,
    pub icon_url: RwSignal<String>,
    pub perks: RwSignal<Vec<String>>,
}

impl WeaponRow {
    fn new(key: u32, w: WeaponForm) -> Self {
        Self {
            key,
            name: RwSignal::new(w.name),
            affinity: RwSignal::new(w.affinity),
            archetype: RwSignal::new(w.archetype),
            icon_url: RwSignal::new(w.icon_url),
            perks: RwSignal::new(w.perks),
        }
    }

    fn to_form(self) -> WeaponForm {
        WeaponForm {
            name: self.name.get_untracked(),
            affinity: self.affinity.get_untracked(),
            archetype: self.archetype.get_untracked(),
            icon_url: self.icon_url.get_untracked(),
            perks: self.perks.get_untracked(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct ArmourRow {
    pub key: u32,
    pub slot: RwSignal<String>,
    pub name: RwSignal<String>,
    pub icon_url: RwSignal<String>,
    pub mods: RwSignal<Vec<String>>,
}

impl ArmourRow {
    fn new(key: u32, a: ArmourForm) -> Self {
        Self {
            key,
            slot: RwSignal::new(a.slot),
            name: RwSignal::new(a.name),
            icon_url: RwSignal::new(a.icon_url),
            mods: RwSignal::new(a.mods),
        }
    }

    fn to_form(self) -> ArmourForm {
        ArmourForm {
            slot: self.slot.get_untracked(),
            name: self.name.get_untracked(),
            icon_url: self.icon_url.get_untracked(),
            mods: self.mods.get_untracked(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct StatRow {
    pub key: u32,
    pub stat: RwSignal<String>,
    pub value: RwSignal<String>,
}

impl StatRow {
    fn new(key: u32, s: StatForm) -> Self {
        Self { key, stat: RwSignal::new(s.stat), value: RwSignal::new(s.value) }
    }

    fn to_form(self) -> StatForm {
        StatForm {
            stat: self.stat.get_untracked(),
            value: self.value.get_untracked(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct EditorState {
    pub id: RwSignal<Option<i32>>,
    pub name: RwSignal<String>,
    pub class: RwSignal<String>,
    pub element: RwSignal<String>,
    pub mode: RwSignal<String>,
    pub super_name: RwSignal<String>,
    pub super_emoji: RwSignal<String>,
    pub class_ability: RwSignal<String>,
    pub jump: RwSignal<String>,
    pub melee: RwSignal<String>,
    pub grenade: RwSignal<String>,
    pub artifact_name: RwSignal<String>,
    pub author: RwSignal<String>,
    pub dim_link: RwSignal<String>,
    pub video_url: RwSignal<String>,
    pub how_it_works: RwSignal<String>,
    pub tags: RwSignal<Vec<String>>,
    pub artifact_perks: RwSignal<Vec<String>>,
    pub aspects: RwSignal<Vec<AspectRow>>,
    pub weapons: RwSignal<Vec<WeaponRow>>,
    pub armour: RwSignal<Vec<ArmourRow>>,
    pub stats: RwSignal<Vec<StatRow>>,
    next_key: RwSignal<u32>,
    owner: StoredValue<Owner>,
}

impl EditorState {
    #[must_use]
    pub fn from_form(f: LoadoutForm) -> Self {
        let state = Self {
            id: RwSignal::new(None),
            name: RwSignal::new(String::new()),
            class: RwSignal::new(String::new()),
            element: RwSignal::new(String::new()),
            mode: RwSignal::new(String::new()),
            super_name: RwSignal::new(String::new()),
            super_emoji: RwSignal::new(String::new()),
            class_ability: RwSignal::new(String::new()),
            jump: RwSignal::new(String::new()),
            melee: RwSignal::new(String::new()),
            grenade: RwSignal::new(String::new()),
            artifact_name: RwSignal::new(String::new()),
            author: RwSignal::new(String::new()),
            dim_link: RwSignal::new(String::new()),
            video_url: RwSignal::new(String::new()),
            how_it_works: RwSignal::new(String::new()),
            tags: RwSignal::new(Vec::new()),
            artifact_perks: RwSignal::new(Vec::new()),
            aspects: RwSignal::new(Vec::new()),
            weapons: RwSignal::new(Vec::new()),
            armour: RwSignal::new(Vec::new()),
            stats: RwSignal::new(Vec::new()),
            next_key: RwSignal::new(0),
            owner: StoredValue::new(Owner::current().unwrap_or_default()),
        };
        state.apply(f);
        state
    }

    pub fn apply(&self, f: LoadoutForm) {
        self.id.set(f.id);
        self.name.set(f.name);
        self.class.set(f.class);
        self.element.set(f.element);
        self.mode.set(f.mode);
        self.super_name.set(f.super_name);
        self.super_emoji.set(f.super_emoji);
        self.class_ability.set(f.class_ability);
        self.jump.set(f.jump);
        self.melee.set(f.melee);
        self.grenade.set(f.grenade);
        self.artifact_name.set(f.artifact_name);
        self.author.set(f.author);
        self.dim_link.set(f.dim_link);
        self.video_url.set(f.video_url);
        self.how_it_works.set(f.how_it_works);
        self.tags.set(f.tags);
        self.artifact_perks.set(f.artifact_perks);
        let (aspects, weapons, armour, stats) = self.in_editor(|| {
            (
                f.aspects
                    .into_iter()
                    .map(|a| AspectRow::new(self.key(), a))
                    .collect(),
                f.weapons
                    .into_iter()
                    .map(|w| WeaponRow::new(self.key(), w))
                    .collect(),
                f.armour
                    .into_iter()
                    .map(|a| ArmourRow::new(self.key(), a))
                    .collect(),
                f.stats.into_iter().map(|s| StatRow::new(self.key(), s)).collect(),
            )
        });
        self.aspects.set(aspects);
        self.weapons.set(weapons);
        self.armour.set(armour);
        self.stats.set(stats);
    }

    fn in_editor<T>(&self, f: impl FnOnce() -> T) -> T {
        self.owner.get_value().with(f)
    }

    #[must_use]
    pub fn to_form(&self) -> LoadoutForm {
        LoadoutForm {
            id: self.id.get_untracked(),
            name: self.name.get_untracked(),
            class: self.class.get_untracked(),
            element: self.element.get_untracked(),
            mode: self.mode.get_untracked(),
            super_name: self.super_name.get_untracked(),
            super_emoji: self.super_emoji.get_untracked(),
            class_ability: self.class_ability.get_untracked(),
            jump: self.jump.get_untracked(),
            melee: self.melee.get_untracked(),
            grenade: self.grenade.get_untracked(),
            artifact_name: self.artifact_name.get_untracked(),
            author: self.author.get_untracked(),
            dim_link: self.dim_link.get_untracked(),
            video_url: self.video_url.get_untracked(),
            how_it_works: self.how_it_works.get_untracked(),
            tags: self.tags.get_untracked(),
            artifact_perks: self.artifact_perks.get_untracked(),
            aspects: self
                .aspects
                .get_untracked()
                .into_iter()
                .map(AspectRow::to_form)
                .collect(),
            weapons: self
                .weapons
                .get_untracked()
                .into_iter()
                .map(WeaponRow::to_form)
                .collect(),
            armour: self
                .armour
                .get_untracked()
                .into_iter()
                .map(ArmourRow::to_form)
                .collect(),
            stats: self
                .stats
                .get_untracked()
                .into_iter()
                .map(StatRow::to_form)
                .collect(),
        }
    }

    #[must_use]
    pub fn snapshot(&self) -> LoadoutForm {
        for s in [
            self.name,
            self.class,
            self.element,
            self.mode,
            self.super_name,
            self.super_emoji,
            self.class_ability,
            self.jump,
            self.melee,
            self.grenade,
            self.artifact_name,
            self.author,
            self.dim_link,
            self.video_url,
            self.how_it_works,
        ] {
            s.track();
        }
        self.tags.track();
        self.artifact_perks.track();
        self.aspects.with(|rows| {
            for r in rows {
                r.aspect.track();
                r.fragments.track();
            }
        });
        self.weapons.with(|rows| {
            for r in rows {
                r.name.track();
                r.affinity.track();
                r.archetype.track();
                r.icon_url.track();
                r.perks.track();
            }
        });
        self.armour.with(|rows| {
            for r in rows {
                r.name.track();
                r.icon_url.track();
                r.mods.track();
            }
        });
        self.stats.with(|rows| {
            for r in rows {
                r.stat.track();
                r.value.track();
            }
        });
        self.to_form()
    }

    fn key(&self) -> u32 {
        let k = self.next_key.get_untracked();
        self.next_key.set(k.wrapping_add(1));
        k
    }

    pub fn add_aspect(&self) {
        let row =
            self.in_editor(|| AspectRow::new(self.key(), AspectForm::default()));
        self.aspects.update(|rows| rows.push(row));
    }

    pub fn remove_aspect(&self, key: u32) {
        self.aspects.update(|rows| rows.retain(|r| r.key != key));
    }

    pub fn add_weapon(&self, blank: WeaponForm) {
        let row = self.in_editor(|| WeaponRow::new(self.key(), blank));
        self.weapons.update(|rows| rows.push(row));
    }

    pub fn move_stat(&self, from: usize, to: usize) {
        self.stats.update(|rows| move_item(rows, from, to));
    }

    pub fn remove_weapon(&self, key: u32) {
        self.weapons.update(|rows| rows.retain(|r| r.key != key));
    }

    #[must_use]
    pub fn emoji_keys(&self) -> Vec<String> {
        let mut keys = vec![
            self.super_emoji.get(),
            self.class_ability.get(),
            self.jump.get(),
            self.melee.get(),
            self.grenade.get(),
        ];
        for a in self.aspects.get() {
            keys.push(a.aspect.get());
            keys.extend(a.fragments.get());
        }
        for w in self.weapons.get() {
            keys.extend(w.perks.get());
        }
        for a in self.armour.get() {
            keys.extend(a.mods.get());
        }
        keys.extend(self.artifact_perks.get());
        keys.retain(|k| !k.trim().is_empty());
        keys
    }
}

pub fn move_item<T>(items: &mut Vec<T>, from: usize, to: usize) {
    if from != to && from < items.len() && to < items.len() {
        let item = items.remove(from);
        items.insert(to, item);
    }
}
