use crate::nav::MODULES;

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

#[must_use]
pub fn aria_current(href: &str, location: &str) -> Option<&'static str> {
    is_active_for(href, location).then_some("page")
}

#[must_use]
pub fn sidebar_active(href: &str, location: &str, exact: bool) -> bool {
    if exact { location == href } else { location.starts_with(href) }
}

#[must_use]
pub fn module_list_location(guild_id: &str, location: &str) -> String {
    let settings = format!("/guild/{guild_id}/settings");
    if location == settings {
        format!("{settings}/general")
    } else {
        location.to_owned()
    }
}

#[must_use]
pub fn switch_href(current: &str, location: &str, target: &str) -> String {
    let overview = format!("/guild/{target}");
    let prefix = format!("/guild/{current}");
    let Some(mut candidate) = location.strip_prefix(prefix.as_str()) else {
        return overview;
    };
    if !candidate.is_empty() && !candidate.starts_with('/') {
        return overview;
    }

    while !candidate.is_empty() {
        let known = candidate == "/settings"
            || MODULES.iter().any(|module| {
                module.href(current).strip_prefix(prefix.as_str()) == Some(candidate)
            });
        if known {
            return format!("{overview}{candidate}");
        }
        match candidate.rsplit_once('/') {
            Some((parent, _)) if !parent.is_empty() => candidate = parent,
            _ => break,
        }
    }

    overview
}
