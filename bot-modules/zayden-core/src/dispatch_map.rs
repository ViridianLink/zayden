use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use crate::IdMatch;

#[derive(Debug)]
pub struct OverlapError {
    pub incoming: Cow<'static, str>,
    pub existing: Cow<'static, str>,
}

impl std::fmt::Display for OverlapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "registry: prefix '{}' overlaps with already-registered prefix '{}'",
            self.incoming, self.existing
        )
    }
}

impl std::error::Error for OverlapError {}

pub struct DispatchMap<T: ?Sized> {
    exact: HashMap<Cow<'static, str>, Arc<T>>,
    prefix: Vec<(Cow<'static, str>, Arc<T>)>,
}

impl<T: ?Sized> Default for DispatchMap<T> {
    fn default() -> Self {
        Self { exact: HashMap::new(), prefix: Vec::new() }
    }
}

impl<T: ?Sized> DispatchMap<T> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(
        &mut self,
        id_match: IdMatch,
        val: Arc<T>,
    ) -> Result<(), OverlapError> {
        match id_match {
            IdMatch::Exact(key) => {
                self.exact.insert(key, val);
            },
            IdMatch::Prefix(key) => {
                for (existing, _) in &self.prefix {
                    if existing != &key
                        && (existing.starts_with(key.as_ref())
                            || key.starts_with(existing.as_ref()))
                    {
                        return Err(OverlapError {
                            incoming: key,
                            existing: existing.clone(),
                        });
                    }
                }

                self.prefix.retain(|(k, _)| k != &key);
                self.prefix.push((key, val));
                self.prefix.sort_by_key(|(k, _)| std::cmp::Reverse(k.len()));
            },
        }
        Ok(())
    }

    #[must_use]
    pub fn lookup(&self, custom_id: &str) -> Option<&Arc<T>> {
        if let Some(v) = self.exact.get(custom_id) {
            return Some(v);
        }
        self.prefix
            .iter()
            .find(|(p, _)| custom_id.starts_with(p.as_ref()))
            .map(|(_, v)| v)
    }
}
