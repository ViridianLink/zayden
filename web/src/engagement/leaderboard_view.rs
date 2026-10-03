use topcoat::context::Cx;
use topcoat::router::request::uri;
use url::form_urlencoded;

pub const GLOBAL_SCOPE: &str = "global";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeaderboardView {
    pub global: bool,
    pub page: i32,
}

impl Default for LeaderboardView {
    fn default() -> Self {
        Self { global: false, page: 1 }
    }
}

impl LeaderboardView {
    /// Reads `?scope=..&page=..` from the request. A repeated key reads as
    /// its first value and an unusable value as the default, so no query
    /// string is refused.
    #[must_use]
    pub fn from_request(cx: &Cx) -> Self {
        Self::from_query_string(uri(cx).query())
    }

    #[must_use]
    pub fn from_query_string(query: Option<&str>) -> Self {
        let mut scope = None;
        let mut page = None;

        for (key, value) in
            form_urlencoded::parse(query.unwrap_or_default().as_bytes())
        {
            match key.as_ref() {
                "scope" if scope.is_none() => scope = Some(value),
                "page" if page.is_none() => page = Some(value),
                _ => {},
            }
        }

        Self::parse(scope.as_deref(), page.as_deref())
    }

    /// `scope` is the global board only when it is exactly `global`. `page` is
    /// a whole number of at least 1, otherwise page 1.
    #[must_use]
    pub fn parse(scope: Option<&str>, page: Option<&str>) -> Self {
        let page = page
            .and_then(|raw| raw.trim().parse::<i32>().ok())
            .map_or(1, |page| page.max(1));

        Self { global: scope == Some(GLOBAL_SCOPE), page }
    }

    /// The address of this view. The default view is the bare path.
    #[must_use]
    pub fn href(self, guild: &str) -> String {
        let base = format!("/guild/{guild}/levels");

        match (self.global, self.page) {
            (false, 1) => base,
            (true, 1) => format!("{base}?scope={GLOBAL_SCOPE}"),
            (false, page) => format!("{base}?page={page}"),
            (true, page) => format!("{base}?scope={GLOBAL_SCOPE}&page={page}"),
        }
    }

    #[must_use]
    pub const fn with_scope(global: bool) -> Self {
        Self { global, page: 1 }
    }

    #[must_use]
    pub fn previous(self) -> Option<Self> {
        (self.page > 1).then(|| Self { page: self.page - 1, ..self })
    }

    #[must_use]
    pub fn next(self, has_next: bool) -> Option<Self> {
        has_next.then(|| Self { page: self.page.saturating_add(1), ..self })
    }

    #[must_use]
    pub const fn shows_pager(self, has_next: bool) -> bool {
        self.page > 1 || has_next
    }

    #[must_use]
    pub const fn empty_text(self) -> &'static str {
        if self.page > 1 {
            "No more entries on this page."
        } else if self.global {
            "No one has earned global XP yet."
        } else {
            "No one has chatted here yet - the board fills as members talk."
        }
    }
}
