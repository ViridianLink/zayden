use crate::faq::index::choice::Target;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) id: i32,
    pub(crate) path: String,
    pub(crate) label: String,
    pub(crate) anchor: Option<String>,
    haystack: String,
}

impl Entry {
    #[must_use]
    pub(crate) fn page(
        id: i32,
        path: String,
        title: String,
        description: &str,
    ) -> Self {
        let haystack = format!("{title} {description} {path}").to_lowercase();

        Self { id, path, label: title, anchor: None, haystack }
    }

    #[must_use]
    pub(crate) fn heading(page: &Self, title: &str, anchor: String) -> Self {
        let label = format!("{} \u{203a} {title}", page.label);
        let haystack =
            format!("{title} {} {}", page.label, page.path).to_lowercase();

        Self {
            id: page.id,
            path: page.path.clone(),
            label,
            anchor: Some(anchor),
            haystack,
        }
    }

    #[must_use]
    pub(crate) fn target(&self) -> Target {
        Target { id: self.id, anchor: self.anchor.clone() }
    }
}

#[must_use]
pub(crate) fn score(entry: &Entry, query: &str) -> usize {
    let query = query.trim().to_lowercase();

    if query.is_empty() {
        // An untouched box gets shallow pages, which are the ones a reader is
        // most likely to want.
        return 1 + 8usize.saturating_sub(entry.path.matches('/').count());
    }

    let label = entry.label.to_lowercase();

    let exact = usize::from(label == query) * 200;
    let prefix = usize::from(label.starts_with(&query)) * 100;
    let whole = usize::from(label.contains(&query)) * 50;

    let terms =
        query.split_whitespace().filter(|term| !term.is_empty()).collect::<Vec<_>>();

    let matched = terms.iter().filter(|term| entry.haystack.contains(*term)).count();

    if matched < terms.len() && exact + prefix + whole == 0 {
        return 0;
    }

    // A page beats one of its own headings when both match equally, so the
    // article itself stays reachable.
    let depth = usize::from(entry.anchor.is_none()) * 2;

    exact + prefix + whole + matched * 10 + depth
}
