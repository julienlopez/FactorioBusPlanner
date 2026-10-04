//! Drawing of the bus: one vertical column per slot.

use std::sync::Arc;

use dioxus::prelude::*;
use fbp_common::{Direction, ItemKind, Slot};

use crate::{
    data::{Data, icon_url},
    state::EditorState,
};

/// How many times a slot's content is repeated along its column.
const ROWS: usize = 4;

#[component]
pub fn BusView() -> Element {
    let state = use_context::<EditorState>();
    let selected = (state.selected)();
    let bus = state.bus.read();
    rsx! {
        main { class: "bus-view",
            div { class: "bus",
                for (index, slot) in bus.slots.iter().enumerate() {
                    SlotColumn {
                        key: "{index}",
                        index,
                        slot: slot.clone(),
                        selected: selected == Some(index),
                    }
                }
            }
        }
    }
}

#[component]
fn SlotColumn(index: usize, slot: Slot, selected: bool) -> Element {
    let data = use_context::<Arc<Data>>();
    let mut state = use_context::<EditorState>();

    let kind = match slot {
        Slot::Empty => "empty",
        Slot::Belt { .. } => "belt",
        Slot::Pipe { .. } => "pipe",
    };
    let arrow = match slot.direction() {
        Some(Direction::Up) => "▲",
        Some(Direction::Down) => "▼",
        None => "",
    };
    let name = |kind, name: &Option<String>| {
        name.as_deref()
            .map_or_else(|| "(empty)".to_owned(), |n| data.display_name(kind, n))
    };
    let tooltip = match &slot {
        Slot::Empty => "Empty".to_owned(),
        Slot::Belt { left, right, .. } if left == right => {
            format!("{} {arrow}", name(ItemKind::Item, left))
        }
        Slot::Belt { left, right, .. } => format!(
            "{} | {} {arrow}",
            name(ItemKind::Item, left),
            name(ItemKind::Item, right)
        ),
        Slot::Pipe { fluid, .. } => format!("{} {arrow}", name(ItemKind::Fluid, fluid)),
    };

    rsx! {
        div {
            class: "slot slot-{kind}",
            class: if selected { "selected" },
            title: "{tooltip}",
            onclick: move |_| state.selected.set(Some(index)),
            div { class: "slot-track",
                for row in 0..ROWS {
                    div { key: "{row}", class: "slot-row",
                        match &slot {
                            Slot::Belt { left, right, .. } => rsx! {
                                ItemIcon { kind: ItemKind::Item, name: left.clone() }
                                ItemIcon { kind: ItemKind::Item, name: right.clone() }
                            },
                            Slot::Pipe { fluid, .. } => rsx! {
                                ItemIcon { kind: ItemKind::Fluid, name: fluid.clone() }
                            },
                            Slot::Empty => rsx! {},
                        }
                    }
                    div { class: "slot-arrow", "{arrow}" }
                }
            }
            div { class: "slot-index", "{index + 1}" }
        }
    }
}

/// The icon of an item or fluid, with placeholders for "nothing" and unknown items.
#[component]
pub fn ItemIcon(kind: ItemKind, name: Option<String>) -> Element {
    let data = use_context::<Arc<Data>>();
    let Some(name) = name else {
        return rsx! { div { class: "icon icon-none" } };
    };
    match data.item(kind, &name) {
        Some(item) => match &item.icon {
            Some(icon) => rsx! {
                img {
                    class: "icon",
                    src: icon_url(icon),
                    title: "{item.localised_name}",
                    draggable: "false",
                }
            },
            None => rsx! {
                div { class: "icon icon-missing", title: "{item.localised_name}", "?" }
            },
        },
        None => rsx! {
            div { class: "icon icon-missing", title: "{name} (not in the extracted data)", "?" }
        },
    }
}
