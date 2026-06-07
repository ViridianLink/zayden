use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use zayden_core::IdMatch;

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

/// Dual-mode lookup table for component and modal handlers.
///
/// Exact registrations always win over prefix registrations.
/// Among prefix registrations, the sorted-longest-first ordering ensures
/// the most-specific match wins (though the overlap invariant makes this
/// a safety measure rather than a tiebreaker — two prefixes may never be
/// a prefix of each other).
pub struct DispatchMap<T: ?Sized> {
    exact: HashMap<Cow<'static, str>, Arc<T>>,
    /// Sorted longest-first so the first matching entry is the most specific.
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

    /// Register a handler.
    ///
    /// For `Exact` keys, an existing registration for the same key is silently
    /// overwritten.  For `Prefix` keys, registering the same prefix again also
    /// overwrites, but registering a prefix that is a prefix-of or
    /// has-as-prefix an *already-registered different* prefix returns
    /// [`OverlapError`] — this is a programmer error caught at startup.
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
                // Allow overwriting the same prefix.
                self.prefix.retain(|(k, _)| k != &key);
                self.prefix.push((key, val));
                // Longest first so the first hit is the most specific.
                self.prefix.sort_by_key(|(k, _)| std::cmp::Reverse(k.len()));
            },
        }
        Ok(())
    }

    /// Look up a handler for `custom_id`.
    ///
    /// Exact match wins; then the longest prefix match is returned.
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

#[cfg(test)]
mod tests {
    use super::*;

    struct Stub(u8);

    fn arc(n: u8) -> Arc<Stub> {
        Arc::new(Stub(n))
    }

    #[test]
    fn exact_match() {
        let mut map: DispatchMap<Stub> = DispatchMap::new();
        let _ = map.insert(IdMatch::Exact(Cow::Borrowed("foo")), arc(1));
        assert_eq!(map.lookup("foo").map(|s| s.0), Some(1));
        assert!(map.lookup("foo_extra").is_none());
    }

    #[test]
    fn prefix_match() {
        let mut map: DispatchMap<Stub> = DispatchMap::new();
        let _ = map.insert(IdMatch::Prefix(Cow::Borrowed("foo_")), arc(1));
        assert_eq!(map.lookup("foo_bar").map(|s| s.0), Some(1));
        assert_eq!(map.lookup("foo_").map(|s| s.0), Some(1));
        assert!(map.lookup("fo").is_none());
        assert!(map.lookup("bar").is_none());
    }

    #[test]
    fn exact_wins_over_prefix() {
        let mut map: DispatchMap<Stub> = DispatchMap::new();
        let _ = map.insert(IdMatch::Prefix(Cow::Borrowed("foo_")), arc(1));
        let _ = map.insert(IdMatch::Exact(Cow::Borrowed("foo_exact")), arc(2));
        assert_eq!(map.lookup("foo_exact").map(|s| s.0), Some(2));
        assert_eq!(map.lookup("foo_other").map(|s| s.0), Some(1));
    }

    #[test]
    fn two_non_overlapping_prefixes() {
        let mut map: DispatchMap<Stub> = DispatchMap::new();
        let _ = map.insert(IdMatch::Prefix(Cow::Borrowed("alpha_")), arc(1));
        let _ = map.insert(IdMatch::Prefix(Cow::Borrowed("beta_")), arc(2));
        assert_eq!(map.lookup("alpha_1").map(|s| s.0), Some(1));
        assert_eq!(map.lookup("beta_2").map(|s| s.0), Some(2));
        assert!(map.lookup("gamma_3").is_none());
    }

    #[test]
    fn same_prefix_overwrites() {
        let mut map: DispatchMap<Stub> = DispatchMap::new();
        let _ = map.insert(IdMatch::Prefix(Cow::Borrowed("foo_")), arc(1));
        let _ = map.insert(IdMatch::Prefix(Cow::Borrowed("foo_")), arc(2));
        assert_eq!(map.lookup("foo_bar").map(|s| s.0), Some(2));
    }

    #[test]
    fn overlapping_prefixes_err() {
        let mut map: DispatchMap<Stub> = DispatchMap::new();
        let _ = map.insert(IdMatch::Prefix(Cow::Borrowed("foo_")), arc(1));
        let result = map.insert(IdMatch::Prefix(Cow::Borrowed("foo_bar_")), arc(2));
        assert!(result.is_err());
    }

    #[test]
    fn no_match_returns_none() {
        let map: DispatchMap<Stub> = DispatchMap::new();
        assert!(map.lookup("anything").is_none());
    }
}
