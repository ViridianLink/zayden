use leptos::prelude::*;

#[component]
pub(crate) fn Skeleton(
    class: &'static str,
    #[prop(default = 1)] count: usize,
) -> impl IntoView {
    (0..count)
        .map(|_| view! { <div class=class aria-hidden="true"></div> })
        .collect_view()
}
