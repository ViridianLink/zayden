use serde_json::Value;

/// `queryKey[0]` for a structured-document ("ST") page, e.g. `weapons/<slug>`,
/// `runners/<slug>`.
pub const ST_DOCUMENT_QUERY_KEY: &str = "ngf-st-document-page";
/// `queryKey[0]` for a user-generated / featured document page, e.g.
/// `builds/<slug>`.
pub const UG_FEATURED_DOCUMENT_QUERY_KEY: &str = "ngf-ug-featured-document-page";

fn key_str(key: &[Value], index: usize) -> Option<&str> {
    key.get(index).and_then(Value::as_str)
}

fn document_query_matches(query_key: &[Value], slug: &str) -> bool {
    key_str(query_key, 0) == Some(ST_DOCUMENT_QUERY_KEY)
        && key_str(query_key, 1) == Some(slug)
}

fn ug_query_matches(query_key: &[Value], category: &str, slug: &str) -> bool {
    key_str(query_key, 0) == Some(UG_FEATURED_DOCUMENT_QUERY_KEY)
        && key_str(query_key, 1) == Some(category)
        && key_str(query_key, 2) == Some(slug)
}

#[must_use]
pub fn find_struct_document<'a>(
    marathon_state: &'a Value,
    slug: &str,
) -> Option<&'a Value> {
    let queries = marathon_state.pointer("/apollo/graphqlV2/queries")?.as_array()?;

    for query in queries {
        let Some(query_key) = query.get("queryKey").and_then(Value::as_array) else {
            continue;
        };
        if !document_query_matches(query_key, slug) {
            continue;
        }

        let Some(entry) =
            query.pointer("/state/data/0/game/documents/structDocumentBySlug")
        else {
            continue;
        };
        if entry.get("error").is_some_and(|e| !e.is_null()) {
            continue;
        }
        if let Some(data) = entry.get("data") {
            return Some(data);
        }
    }

    None
}

#[must_use]
pub fn find_ug_document<'a>(
    marathon_state: &'a Value,
    category: &str,
    slug: &str,
) -> Option<&'a Value> {
    let queries = marathon_state.pointer("/apollo/graphqlV2/queries")?.as_array()?;

    for query in queries {
        let Some(query_key) = query.get("queryKey").and_then(Value::as_array) else {
            continue;
        };
        if !ug_query_matches(query_key, category, slug) {
            continue;
        }

        let Some(entry) = query
            .pointer("/state/data/0/game/documents/userGeneratedDocumentBySlug")
        else {
            continue;
        };
        if entry.get("error").is_some_and(|e| !e.is_null()) {
            continue;
        }
        if let Some(data) = entry.get("data") {
            return Some(data);
        }
    }

    None
}
