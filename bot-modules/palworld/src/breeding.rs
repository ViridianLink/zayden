use std::collections::HashMap;

use crate::model::ParentPair;
use crate::transport::BreedingMap;

fn norm(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.to_string(), b.to_string())
    } else {
        (b.to_string(), a.to_string())
    }
}

#[derive(Debug, Default)]
pub struct BreedingIndex {
    forward: HashMap<(String, String), String>,
    reverse: BreedingMap,
}

impl BreedingIndex {
    #[must_use]
    pub fn from_map(map: BreedingMap) -> Self {
        let mut forward: HashMap<(String, String), String> = HashMap::new();
        for (child, pairs) in &map {
            for pair in pairs {
                let (a, b) = (&pair[0], &pair[1]);
                forward.entry(norm(a, b)).or_insert_with(|| child.clone());
            }
        }
        Self { forward, reverse: map }
    }

    #[must_use]
    pub fn breed(&self, a: &str, b: &str) -> Option<&str> {
        self.forward.get(&norm(a, b)).map(String::as_str)
    }

    #[must_use]
    pub fn is_unique_child(&self, target: &str) -> bool {
        self.reverse.get(target).is_some_and(|pairs| pairs.len() == 1)
    }

    #[must_use]
    pub fn breed_for(&self, target: &str) -> Vec<ParentPair> {
        self.reverse
            .get(target)
            .map(|pairs| {
                pairs
                    .iter()
                    .map(|p| ParentPair { a: p[0].clone(), b: p[1].clone() })
                    .collect()
            })
            .unwrap_or_default()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.forward.is_empty()
    }
}
