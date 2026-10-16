use deadlock_api::model::Hero;

#[derive(Debug, Clone)]
pub struct HeroIndex {
    heroes: Vec<Hero>,
}

impl HeroIndex {
    #[must_use]
    pub fn new(heroes: &[Hero]) -> Self {
        let mut heroes: Vec<Hero> = heroes
            .iter()
            .filter(|hero| {
                hero.player_selectable && !hero.disabled && !hero.in_development
            })
            .cloned()
            .collect();
        heroes.sort_by(|a, b| a.name.cmp(&b.name));

        Self { heroes }
    }

    #[must_use]
    pub fn get(&self, id: u32) -> Option<&Hero> {
        self.heroes.iter().find(|hero| hero.id == id)
    }

    #[must_use]
    pub fn find(&self, query: &str) -> Option<&Hero> {
        let query = query.trim();

        if let Ok(id) = query.parse::<u32>() {
            return self.get(id);
        }

        let lowered = query.to_lowercase();
        let prefixed = format!("hero_{lowered}");

        self.heroes
            .iter()
            .find(|hero| hero.name.to_lowercase() == lowered)
            .or_else(|| self.heroes.iter().find(|hero| hero.class_name == query))
            .or_else(|| self.heroes.iter().find(|hero| hero.class_name == prefixed))
    }

    #[must_use]
    pub fn search(&self, query: &str, limit: usize) -> Vec<&Hero> {
        let needle = query.to_lowercase();

        if needle.is_empty() {
            return self.heroes.iter().take(limit).collect();
        }

        let mut starts_with = Vec::new();
        let mut contains = Vec::new();

        for hero in &self.heroes {
            let name = hero.name.to_lowercase();
            if name.starts_with(&needle) {
                starts_with.push(hero);
            } else if name.contains(&needle) {
                contains.push(hero);
            }
        }

        starts_with.extend(contains);
        starts_with.truncate(limit);
        starts_with
    }

    pub fn iter(&self) -> impl Iterator<Item = &Hero> {
        self.heroes.iter()
    }
}
