use std::borrow::Cow;
use std::collections::HashMap;

use serenity::all::{
    ActionRowComponent,
    Component,
    ContainerComponent,
    LabelComponent,
    ModalComponent,
    SectionComponent,
};
use tracing::warn;

#[must_use]
pub fn parse_modal_components(
    components: &[ModalComponent],
) -> HashMap<Cow<'_, str>, Vec<Cow<'_, str>>> {
    components
        .iter()
        .filter_map(|component| match component {
            ModalComponent::TextDisplay(_) => None,
            ModalComponent::Label(label) => {
                Some(vec![parse_label(&label.component)])
            },
            c => {
                warn!("New component {c:?}");
                None
            },
        })
        .flatten()
        .collect()
}

#[must_use]
pub fn parse_text_components(
    components: &[Component],
) -> HashMap<Cow<'_, str>, Vec<Cow<'_, str>>> {
    components
        .iter()
        .enumerate()
        .filter_map(|(i, component)| match component {
            Component::ActionRow(action_row) => {
                Some(parse_action_row(&action_row.components))
            },
            Component::Section(section) => Some(parse_section(&section.components)),
            Component::TextDisplay(_)
            | Component::MediaGallery(_)
            | Component::Separator(_)
            | Component::Unknown(_) => None,
            Component::File(file_component) => Some(vec![(
                Cow::Owned(format!("file_{i}")),
                vec![Cow::Borrowed(&file_component.file.url)],
            )]),
            Component::Container(container) => {
                Some(parse_container(&container.components))
            },
            Component::Label(label) => Some(vec![parse_label(&label.component)]),
            c => {
                warn!("New component {c:?}");
                None
            },
        })
        .flatten()
        .collect()
}

fn parse_action_row(
    components: &[ActionRowComponent],
) -> Vec<(Cow<'_, str>, Vec<Cow<'_, str>>)> {
    components
        .iter()
        .filter_map(|c| match c {
            ActionRowComponent::Button(_) => None,
            ActionRowComponent::SelectMenu(_select_menu) => {
                warn!(
                    "parse_action_row: SelectMenu encountered; multi-value select menus are not yet supported and will be skipped",
                );
                None
            }
            c => {
                warn!("New action row component {c:?}");
                None
            }
        })
        .collect()
}

fn parse_section(
    components: &[SectionComponent],
) -> Vec<(Cow<'_, str>, Vec<Cow<'_, str>>)> {
    components
        .iter()
        .filter_map(|c| match c {
            SectionComponent::TextDisplay(_) => None,
            c => {
                warn!("New component: {c:?}");
                None
            },
        })
        .collect()
}

fn parse_container(
    _components: &[ContainerComponent],
) -> Vec<(Cow<'_, str>, Vec<Cow<'_, str>>)> {
    warn!("parse_container: container parsing not yet implemented; returning empty");
    Vec::new()
}

fn parse_label(component: &LabelComponent) -> (Cow<'_, str>, Vec<Cow<'_, str>>) {
    match component {
        LabelComponent::SelectMenu(select_menu) => (
            Cow::Borrowed(&select_menu.custom_id),
            select_menu.values.iter().map(Cow::from).collect(),
        ),
        LabelComponent::InputText(input_text) => (
            Cow::Borrowed(&input_text.custom_id),
            vec![Cow::Borrowed(&input_text.value)],
        ),
        LabelComponent::FileUpload(file_upload) => (
            Cow::Borrowed(&file_upload.custom_id),
            file_upload.values.iter().map(|id| Cow::Owned(id.to_string())).collect(),
        ),
        LabelComponent::RadioGroup(_)
        | LabelComponent::CheckboxGroup(_)
        | LabelComponent::Checkbox(_) => {
            warn!("parse_label: component uses a non-text format; skipping",);
            (Cow::Owned(String::new()), Vec::new())
        },

        c => {
            warn!("New label component {c:?}");
            (Cow::Owned(String::new()), vec![Cow::Owned(String::new())])
        },
    }
}
