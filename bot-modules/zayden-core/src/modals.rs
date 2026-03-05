use serenity::all::{
    ActionRowComponent, Component, ContainerComponent, LabelComponent, ModalComponent,
    SectionComponent,
};
use std::borrow::Cow;
use std::collections::HashMap;
use tracing::warn;

pub fn parse_modal_components(
    components: &[ModalComponent],
) -> HashMap<Cow<'_, str>, Vec<Cow<'_, str>>> {
    components
        .iter()
        .filter_map(|component| match component {
            ModalComponent::TextDisplay(_) => None,
            ModalComponent::Label(label) => Some(vec![parse_label(&label.component)]),
            c => {
                warn!("New component {c:?}");
                None
            }
        })
        .flatten()
        .collect()
}

pub fn parse_text_components(components: &[Component]) -> HashMap<Cow<'_, str>, Vec<Cow<'_, str>>> {
    components
        .iter()
        .enumerate()
        .filter_map(|(i, component)| match component {
            Component::ActionRow(action_row) => Some(parse_action_row(&action_row.components)),
            Component::Section(section) => Some(parse_section(&section.components)),
            Component::TextDisplay(_) => None,
            Component::MediaGallery(_) => None,
            Component::File(file_component) => Some(vec![(
                Cow::Owned(format!("file_{i}")),
                vec![Cow::Borrowed(&file_component.file.url)],
            )]),
            Component::Separator(_) => None,
            Component::Container(container) => Some(parse_container(&container.components)),
            Component::Label(label) => Some(vec![parse_label(&label.component)]),
            Component::Unknown(_) => None,
            c => {
                warn!("New component {c:?}");
                None
            }
        })
        .flatten()
        .collect()
}

fn parse_action_row(components: &[ActionRowComponent]) -> Vec<(Cow<'_, str>, Vec<Cow<'_, str>>)> {
    components
        .iter()
        .filter_map(|c| match c {
            ActionRowComponent::Button(_) => None,
            ActionRowComponent::SelectMenu(select_menu) => {
                todo!("Select menu can have multiple values which breaks current function")
            }
            c => {
                warn!("New action row component {c:?}");
                None
            }
        })
        .collect()
}

fn parse_section(components: &[SectionComponent]) -> Vec<(Cow<'_, str>, Vec<Cow<'_, str>>)> {
    components
        .iter()
        .filter_map(|c| match c {
            SectionComponent::TextDisplay(_) => None,
            c => {
                warn!("New component: {c:?}");
                None
            }
        })
        .collect()
}

fn parse_container(components: &[ContainerComponent]) -> Vec<(Cow<'_, str>, Vec<Cow<'_, str>>)> {
    todo!()
}

fn parse_label(component: &LabelComponent) -> (Cow<'_, str>, Vec<Cow<'_, str>>) {
    match component {
        LabelComponent::SelectMenu(select_menu) => (
            Cow::Borrowed(&select_menu.custom_id),
            select_menu
                .values
                .iter()
                .map(Cow::from) // Creates Cow::Borrowed(&str)
                .collect(),
        ),
        LabelComponent::InputText(input_text) => (
            Cow::Borrowed(&input_text.custom_id),
            vec![Cow::Borrowed(
                input_text.value.as_deref().unwrap_or_default(),
            )],
        ),
        LabelComponent::FileUpload(file_upload) => {
            panic!("File upload uses a non-text format");
        }
        c => {
            warn!("New label component {c:?}");
            (Cow::Owned(String::new()), vec![Cow::Owned(String::new())])
        }
    }
}
