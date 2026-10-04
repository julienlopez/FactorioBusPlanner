//! Sidebar: editing the selected slot and picking items for it.

use std::sync::Arc;

use dioxus::prelude::*;
use fbp_common::{Direction, Item, ItemKind, Slot};

use crate::{
    bus_view::ItemIcon,
    data::{Data, icon_url},
    state::{EditorState, Side},
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotKind {
    Empty,
    Belt,
    Pipe,
}

impl SlotKind {
    fn of(slot: &Slot) -> Self {
        match slot {
            Slot::Empty => SlotKind::Empty,
            Slot::Belt { .. } => SlotKind::Belt,
            Slot::Pipe { .. } => SlotKind::Pipe,
        }
    }

    /// A blank slot of this kind, keeping the previous direction if there was one.
    fn blank(self, previous: &Slot) -> Slot {
        let direction = previous.direction().unwrap_or_default();
        match self {
            SlotKind::Empty => Slot::Empty,
            SlotKind::Belt => Slot::Belt {
                left: None,
                right: None,
                direction,
            },
            SlotKind::Pipe => Slot::Pipe {
                fluid: None,
                direction,
            },
        }
    }
}

#[component]
pub fn SlotEditor() -> Element {
    let mut state = use_context::<EditorState>();
    let Some(index) = (state.selected)() else {
        return rsx! { p { class: "hint", "Select a slot to edit it." } };
    };
    let Some(slot) = state.bus.read().slots.get(index).cloned() else {
        return rsx! {};
    };
    let kind = SlotKind::of(&slot);

    rsx! {
        section { class: "panel",
            h2 { "Slot {index + 1}" }
            div { class: "segmented",
                for (label, option) in [("Empty", SlotKind::Empty), ("Belt", SlotKind::Belt), ("Pipe", SlotKind::Pipe)] {
                    button {
                        class: if kind == option { "active" },
                        onclick: move |_| state.update_selected(|slot| {
                            if SlotKind::of(slot) != option {
                                *slot = option.blank(slot);
                            }
                        }),
                        "{label}"
                    }
                }
            }
            if let Some(direction) = slot.direction() {
                div { class: "segmented",
                    for (label, option) in [("▲ Up", Direction::Up), ("▼ Down", Direction::Down)] {
                        button {
                            class: if direction == option { "active" },
                            onclick: move |_| state.update_selected(|slot| {
                                if let Slot::Belt { direction, .. } | Slot::Pipe { direction, .. } = slot {
                                    *direction = option;
                                }
                            }),
                            "{label}"
                        }
                    }
                }
            }
            match &slot {
                Slot::Belt { left, right, .. } => rsx! {
                    BeltSides { left: left.clone(), right: right.clone() }
                },
                Slot::Pipe { fluid, .. } => rsx! {
                    Assignment {
                        label: "Fluid",
                        kind: ItemKind::Fluid,
                        name: fluid.clone(),
                        onclear: move |_| state.update_selected(|slot| {
                            if let Slot::Pipe { fluid, .. } = slot {
                                *fluid = None;
                            }
                        }),
                    }
                },
                Slot::Empty => rsx! {},
            }
            div { class: "actions",
                button { onclick: move |_| state.move_selected(-1), "◀ Move" }
                button { onclick: move |_| state.move_selected(1), "Move ▶" }
                button { onclick: move |_| state.insert_at(index), "Insert left" }
                button { onclick: move |_| state.insert_at(index + 1), "Insert right" }
                button { class: "danger", onclick: move |_| state.remove_selected(), "Delete" }
            }
        }
        match kind {
            SlotKind::Belt => rsx! { ItemPicker { kind: ItemKind::Item } },
            SlotKind::Pipe => rsx! { ItemPicker { kind: ItemKind::Fluid } },
            SlotKind::Empty => rsx! {},
        }
    }
}

#[component]
fn BeltSides(left: Option<String>, right: Option<String>) -> Element {
    let mut state = use_context::<EditorState>();
    let side = (state.side)();
    let mut clear = move |clear_left: bool| {
        state.update_selected(|slot| {
            if let Slot::Belt { left, right, .. } = slot {
                *(if clear_left { left } else { right }) = None;
            }
        })
    };

    rsx! {
        div { class: "segmented",
            span { class: "label", "Assign to" }
            for (label, option) in [("Both", Side::Both), ("Left", Side::Left), ("Right", Side::Right)] {
                button {
                    class: if side == option { "active" },
                    onclick: move |_| state.side.set(option),
                    "{label}"
                }
            }
        }
        Assignment {
            label: "Left",
            kind: ItemKind::Item,
            name: left,
            onclear: move |_| clear(true),
        }
        Assignment {
            label: "Right",
            kind: ItemKind::Item,
            name: right,
            onclear: move |_| clear(false),
        }
        button {
            onclick: move |_| state.update_selected(|slot| {
                if let Slot::Belt { left, right, .. } = slot {
                    std::mem::swap(left, right);
                }
            }),
            "⇄ Swap sides"
        }
    }
}

/// One assigned item or fluid, with a button to clear it.
#[component]
fn Assignment(
    label: &'static str,
    kind: ItemKind,
    name: Option<String>,
    onclear: EventHandler<()>,
) -> Element {
    let data = use_context::<Arc<Data>>();
    let display = name
        .as_deref()
        .map_or_else(|| "(empty)".to_owned(), |n| data.display_name(kind, n));
    rsx! {
        div { class: "assignment",
            span { class: "label", "{label}" }
            ItemIcon { kind, name: name.clone() }
            span { class: "name", "{display}" }
            if name.is_some() {
                button { class: "clear", title: "Clear", onclick: move |_| onclear.call(()), "✕" }
            }
        }
    }
}

/// Item grid laid out like the in-game crafting menu: group tabs, one row per subgroup.
#[component]
fn ItemPicker(kind: ItemKind) -> Element {
    let data = use_context::<Arc<Data>>();
    let mut state = use_context::<EditorState>();
    let mut search = use_signal(String::new);
    let mut chosen_group = use_signal(|| None::<String>);

    let candidates: Vec<&Item> = data
        .catalog
        .items
        .iter()
        .filter(|item| item.kind == kind)
        .collect();
    let groups: Vec<_> = data
        .catalog
        .groups
        .iter()
        .filter(|group| candidates.iter().any(|item| data.group_of(item) == group.name))
        .collect();
    let active_group = chosen_group
        .read()
        .clone()
        .filter(|name| groups.iter().any(|g| &g.name == name))
        .or_else(|| groups.first().map(|g| g.name.clone()));

    let query = search.read().to_lowercase();
    let shown = candidates.into_iter().filter(|item| {
        if query.is_empty() {
            active_group.as_deref() == Some(data.group_of(item))
        } else {
            item.localised_name.to_lowercase().contains(&query) || item.name.contains(&query)
        }
    });
    // Items are already sorted by subgroup, so each subgroup is a contiguous run.
    let mut rows: Vec<Vec<&Item>> = Vec::new();
    for item in shown {
        match rows.last_mut() {
            Some(row) if row[0].subgroup == item.subgroup => row.push(item),
            _ => rows.push(vec![item]),
        }
    }

    rsx! {
        section { class: "panel picker",
            input {
                r#type: "search",
                placeholder: if kind == ItemKind::Fluid { "Search fluids…" } else { "Search items…" },
                value: "{search}",
                oninput: move |e| search.set(e.value()),
            }
            if query.is_empty() {
                div { class: "group-tabs",
                    for group in groups {
                        button {
                            key: "{group.name}",
                            class: if active_group.as_deref() == Some(&group.name) { "active" },
                            title: "{group.localised_name}",
                            onclick: {
                                let name = group.name.clone();
                                move |_| chosen_group.set(Some(name.clone()))
                            },
                            if let Some(icon) = &group.icon {
                                img { src: icon_url(icon), draggable: "false" }
                            } else {
                                "{group.localised_name}"
                            }
                        }
                    }
                }
            }
            div { class: "picker-grid",
                if rows.is_empty() {
                    p { class: "hint", "Nothing matches." }
                }
                for row in rows {
                    div { key: "{row[0].subgroup}", class: "picker-row",
                        for item in row {
                            button {
                                key: "{item.name}",
                                class: "picker-item",
                                title: "{item.localised_name}",
                                onclick: {
                                    let name = item.name.clone();
                                    move |_| state.assign(name.clone())
                                },
                                ItemIcon { kind, name: Some(item.name.clone()) }
                            }
                        }
                    }
                }
            }
        }
    }
}
