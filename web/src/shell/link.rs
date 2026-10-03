//! Link state the dashboard derives from the request path.

/// Whether a link to `href` marks the page at `location` as current: the
/// location is `href` itself or nested under it, compared segment by segment,
/// and a trailing slash on `href` is optional.
#[must_use]
pub fn is_active_for(href: &str, location: &str) -> bool {
    let mut href_segments = href.split('/');

    std::iter::zip(location.split('/'), href_segments.by_ref()).enumerate().all(
        |(index, (location_segment, href_segment))| {
            location_segment == href_segment
                || (href_segment.is_empty() && index > 1)
        },
    ) && matches!(href_segments.next(), None | Some(""))
}

/// The `aria-current` value of a link to `href` on the page at `location`.
#[must_use]
pub fn aria_current(href: &str, location: &str) -> Option<&'static str> {
    is_active_for(href, location).then_some("page")
}

/// A sidebar entry's state: `exact` entries are active only on their own
/// page, the others on any page whose path starts with `href`.
#[must_use]
pub fn sidebar_active(href: &str, location: &str, exact: bool) -> bool {
    if exact { location == href } else { location.starts_with(href) }
}

/// The path the module list highlights: the bare settings page shows the
/// General section, so it counts as `/guild/{guild_id}/settings/general`.
#[must_use]
pub fn module_list_location(guild_id: &str, location: &str) -> String {
    let settings = format!("/guild/{guild_id}/settings");
    if location == settings {
        format!("{settings}/general")
    } else {
        location.to_owned()
    }
}
