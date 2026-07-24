//! Routing behaviour of the component/modal [`DispatchMap`]: exact-over-prefix
//! precedence, longest-prefix matching, same-prefix overwrite, and the
//! startup overlap invariant that rejects two prefixes that nest.

use std::borrow::Cow;
use std::sync::Arc;

use zayden_core::{DispatchMap, IdMatch};

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
