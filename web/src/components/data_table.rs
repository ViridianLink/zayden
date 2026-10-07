#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{Child, View, component, view};

#[component]
pub async fn data_table(
    caption: &str,
    columns: &[&str],
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div class="data-table-wrap">
            <table class="data-table" role="table">
                <caption class="visually-hidden">(caption)</caption>
                <thead role="rowgroup">
                    <tr role="row">
                        #[key(column)]
                        for column in columns {
                            <th scope="col" role="columnheader">(*column)</th>
                        }
                    </tr>
                </thead>
                <tbody role="rowgroup">(child)</tbody>
            </table>
        </div>
    })
}

#[component]
pub async fn data_row(#[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! { <tr role="row">(child)</tr> })
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
            <th scope="row" role="rowheader" data-label=(label) class=(class)>
                (child)
            </th>
        } else {
            <td role="cell" data-label=(label) class=(class)>(child)</td>
        }
    })
}
