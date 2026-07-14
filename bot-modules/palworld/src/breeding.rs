use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

use crate::model::{BreedPlan, BreedStep, Gender, OwnedPal, ParentPair};
use crate::transport::BreedingMap;

const BREED_STEP: i64 = 1;

const MAX_RECONSTRUCT_OPS: usize = 100_000;

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

    #[must_use]
    pub fn plan(
        &self,
        owned: &[OwnedPal],
        target: &str,
        base_cost: &HashMap<String, i64>,
    ) -> Option<BreedPlan> {
        let mut owned_species: HashSet<&str> = HashSet::new();
        let mut genders: HashMap<&str, (bool, bool)> = HashMap::new();
        for pal in owned {
            owned_species.insert(pal.species.as_str());
            let entry =
                genders.entry(pal.species.as_str()).or_insert((false, false));
            match pal.gender {
                Gender::Male => entry.0 = true,
                Gender::Female => entry.1 = true,
                Gender::Unknown => {},
            }
        }

        let mut incident: HashMap<&str, Vec<(&str, &str, &str)>> = HashMap::new();
        for (child, pairs) in &self.reverse {
            for pair in pairs {
                let (a, b, c) = (pair[0].as_str(), pair[1].as_str(), child.as_str());
                incident.entry(a).or_default().push((b, a, c));
                if a != b {
                    incident.entry(b).or_default().push((a, a, c));
                }
            }
        }

        let mut dist: HashMap<&str, (i64, i64)> = HashMap::new();
        let mut via: HashMap<&str, Option<(&str, &str)>> = HashMap::new();
        let mut finalized: HashMap<&str, i64> = HashMap::new();
        let mut heap: BinaryHeap<Reverse<(i64, i64, &str)>> = BinaryHeap::new();

        for &species in &owned_species {
            dist.insert(species, (0, i64::MIN));
            via.insert(species, None);
            heap.push(Reverse((0, i64::MIN, species)));
        }
        for (species, &cost) in base_cost {
            let s = species.as_str();
            if owned_species.contains(s) {
                continue;
            }
            dist.insert(s, (cost, i64::MIN));
            via.insert(s, None);
            heap.push(Reverse((cost, i64::MIN, s)));
        }

        while let Some(Reverse((cost, second, s))) = heap.pop() {
            if finalized.contains_key(s) || dist.get(s) != Some(&(cost, second)) {
                continue;
            }
            finalized.insert(s, cost);

            let Some(list) = incident.get(s) else { continue };
            for &(partner, first_parent, child) in list {
                if finalized.contains_key(child) {
                    continue;
                }
                let Some(&partner_cost) = finalized.get(partner) else {
                    continue;
                };
                let cand = cost + partner_cost + BREED_STEP;
                let tie = cost.max(partner_cost);
                let better = match dist.get(child) {
                    Some(&existing) => (cand, tie) < existing,
                    None => true,
                };
                if better {
                    let pair =
                        if first_parent == s { (s, partner) } else { (partner, s) };
                    dist.insert(child, (cand, tie));
                    via.insert(child, Some(pair));
                    heap.push(Reverse((cand, tie, child)));
                }
            }
        }

        let ready = |a: &str, b: &str| -> bool {
            let ga = genders.get(a).copied().unwrap_or((false, false));
            let gb = genders.get(b).copied().unwrap_or((false, false));
            if a == b { ga.0 && ga.1 } else { (ga.0 && gb.1) || (ga.1 && gb.0) }
        };

        if owned_species.contains(target) {
            return self.plan_extra_copy(
                target,
                genders.get(target).copied().unwrap_or((false, false)),
                &finalized,
                &via,
                &owned_species,
                &ready,
            );
        }

        let &total_cost = finalized.get(target)?;
        let (order, leaves) = reconstruct(&via, &owned_species, &[target])?;

        Some(BreedPlan {
            target: target.to_string(),
            steps: build_steps(&order, &via, &ready),
            total_cost,
            leaves_to_obtain: leaves.iter().map(|s| (*s).to_string()).collect(),
        })
    }

    fn plan_extra_copy<'a>(
        &'a self,
        target: &'a str,
        owned_genders: (bool, bool),
        finalized: &HashMap<&'a str, i64>,
        via: &HashMap<&'a str, Option<(&'a str, &'a str)>>,
        owned_species: &HashSet<&'a str>,
        ready: &impl Fn(&str, &str) -> bool,
    ) -> Option<BreedPlan> {
        let (has_male, has_female) = owned_genders;
        let has_self = self.breed(target, target) == Some(target);

        let self_step = |ready: bool| BreedPlan {
            target: target.to_string(),
            steps: vec![BreedStep {
                pair: ParentPair { a: target.to_string(), b: target.to_string() },
                child: target.to_string(),
                ready,
            }],
            total_cost: BREED_STEP,
            leaves_to_obtain: Vec::new(),
        };

        if has_self && has_male && has_female {
            return Some(self_step(true));
        }

        let best = self
            .reverse
            .get(target)?
            .iter()
            .filter(|pair| !(pair[0] == target && pair[1] == target))
            .filter_map(|pair| {
                let (a, b) = (pair[0].as_str(), pair[1].as_str());
                let cost = finalized.get(a)? + finalized.get(b)? + BREED_STEP;
                Some((cost, a, b))
            })
            .min();

        if let Some((total_cost, a, b)) = best {
            let (order, leaves) = reconstruct(via, owned_species, &[a, b])?;
            let mut steps = build_steps(&order, via, ready);
            steps.push(BreedStep {
                pair: ParentPair { a: a.to_string(), b: b.to_string() },
                child: target.to_string(),
                ready: ready(a, b),
            });
            return Some(BreedPlan {
                target: target.to_string(),
                steps,
                total_cost,
                leaves_to_obtain: leaves.iter().map(|s| (*s).to_string()).collect(),
            });
        }

        has_self.then(|| self_step(false))
    }
}

fn reconstruct<'a>(
    via: &HashMap<&'a str, Option<(&'a str, &'a str)>>,
    owned_species: &HashSet<&'a str>,
    roots: &[&'a str],
) -> Option<(Vec<&'a str>, Vec<&'a str>)> {
    let mut order: Vec<&str> = Vec::new();
    let mut leaves: Vec<&str> = Vec::new();
    let mut visited: HashSet<&str> = HashSet::new();
    let mut stack: Vec<(&str, bool)> =
        roots.iter().rev().map(|&r| (r, false)).collect();
    let mut ops = 0usize;
    while let Some((node, expanded)) = stack.pop() {
        ops += 1;
        if ops > MAX_RECONSTRUCT_OPS {
            return None;
        }
        if expanded {
            order.push(node);
            continue;
        }
        if !visited.insert(node) {
            continue;
        }
        match via.get(node) {
            Some(&Some((a, b))) => {
                stack.push((node, true));
                stack.push((a, false));
                if b != a {
                    stack.push((b, false));
                }
            },
            _ => {
                if !owned_species.contains(node) {
                    leaves.push(node);
                }
            },
        }
    }
    Some((order, leaves))
}

fn build_steps<'a>(
    order: &[&'a str],
    via: &HashMap<&'a str, Option<(&'a str, &'a str)>>,
    ready: &impl Fn(&str, &str) -> bool,
) -> Vec<BreedStep> {
    order
        .iter()
        .filter_map(|&node| {
            let Some(&Some((a, b))) = via.get(node) else {
                return None;
            };
            Some(BreedStep {
                pair: ParentPair { a: a.to_string(), b: b.to_string() },
                child: node.to_string(),
                ready: ready(a, b),
            })
        })
        .collect()
}
