pub(crate) use zayden_core::text::close_fence;

pub(crate) const ELLIPSIS: &str = "\n\n_(truncated - read the full article via the title \
                            link)_";

#[must_use]
pub fn truncate(content: &str, limit: usize) -> String {
    zayden_core::text::truncate(content, limit, ELLIPSIS)
}
