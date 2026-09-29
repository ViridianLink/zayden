use std::sync::atomic::{AtomicUsize, Ordering};

use leptos::ev::{DragEvent, KeyboardEvent};
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::HtmlElement;

static NEXT_LIST: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy)]
pub struct Reorder {
    list: StoredValue<String>,
    dragging: StoredValue<Option<usize>>,
    over: RwSignal<Option<usize>>,
    on_move: Callback<(usize, usize)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Row,
    Column,
}

impl Reorder {
    pub fn new(on_move: impl Fn(usize, usize) + Send + Sync + 'static) -> Self {
        let n = NEXT_LIST.fetch_add(1, Ordering::Relaxed);
        Self {
            list: StoredValue::new(format!("reorder-{n}")),
            dragging: StoredValue::new(None),
            over: RwSignal::new(None),
            on_move: Callback::new(move |(from, to)| on_move(from, to)),
        }
    }

    #[must_use]
    pub fn item_id(&self, index: usize) -> String {
        format!("{}-{index}", self.list.get_value())
    }

    #[must_use]
    pub fn is_target(&self, index: usize) -> bool {
        self.over.get() == Some(index)
    }

    pub fn start(&self, index: usize, ev: &DragEvent) {
        self.dragging.set_value(Some(index));
        allow_move(ev);
    }

    pub fn over(&self, index: usize, ev: &DragEvent) {
        if self.dragging.get_value().is_some() {
            ev.prevent_default();
            if self.over.get_untracked() != Some(index) {
                self.over.set(Some(index));
            }
        }
    }

    pub fn drop(&self, index: usize, ev: &DragEvent) {
        ev.prevent_default();
        if let Some(from) = self.dragging.get_value() {
            self.on_move.run((from, index));
        }
        self.end();
    }

    pub fn end(&self) {
        self.dragging.set_value(None);
        self.over.set(None);
    }

    pub fn key(
        &self,
        index: usize,
        len: usize,
        axis: Axis,
        alt: bool,
        ev: &KeyboardEvent,
    ) {
        if alt && !ev.alt_key() {
            return;
        }
        let (back, forward) = match axis {
            Axis::Row => ("ArrowLeft", "ArrowRight"),
            Axis::Column => ("ArrowUp", "ArrowDown"),
        };
        let to = match ev.key().as_str() {
            k if k == back => index.checked_sub(1),
            k if k == forward => (index + 1 < len).then_some(index + 1),
            _ => return,
        };
        ev.prevent_default();
        if let Some(to) = to {
            self.on_move.run((index, to));
            let id = self.item_id(to);
            request_animation_frame(move || {
                if let Some(el) = document()
                    .get_element_by_id(&id)
                    .and_then(|e| e.dyn_into::<HtmlElement>().ok())
                {
                    let _ = el.focus();
                }
            });
        }
    }
}

#[cfg(feature = "hydrate")]
fn allow_move(ev: &DragEvent) {
    if let Some(transfer) = ev.data_transfer() {
        transfer.set_effect_allowed("move");
        let _ = transfer.set_data("text/plain", "");
    }
}

#[cfg(not(feature = "hydrate"))]
const fn allow_move(_ev: &DragEvent) {}
