#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{Child, View, attributes, component, view};

use super::table::{
    table,
    table_body,
    table_caption,
    table_cell,
    table_head,
    table_header,
    table_row,
};

#[component]
pub async fn data_table(
    caption: &str,
    columns: &[&str],
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        table(
            attrs: attributes! { class="data-table" role="table" },
            table_caption(attrs: attributes! { class="visually-hidden" }, (caption))
            table_header(
                attrs: attributes! { role="rowgroup" },
                table_row(
                    attrs: attributes! { role="row" },
                    #[key(column)]
                    for column in columns {
                        table_head(
                            attrs: attributes! { scope="col" role="columnheader" },
                            (*column)
                        )
                    }
                )
            )
            table_body(attrs: attributes! { role="rowgroup" }, (child))
        )
    })
}

#[component]
pub async fn data_row(#[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! { table_row(attrs: attributes! { role="row" }, (child)) })
}

#[component]
pub async fn data_cell(
    label: &str,
    #[default] header: bool,
    #[default] numeric: bool,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let class = numeric.then_some("num");

    Ok(view! {
        if header {
            table_head(
                attrs: attributes! {
                    scope="row"
                    role="rowheader"
                    data-label=(label)
                    class=(class)
                },
                (child)
            )
        } else {
            table_cell(
                attrs: attributes! { role="cell" data-label=(label) class=(class) },
                (child)
            )
        }
    })
}
