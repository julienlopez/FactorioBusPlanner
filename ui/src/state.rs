//! Editor state shared by all components, and the actions that modify it.

use std::path::PathBuf;

use dioxus::prelude::*;
use fbp_common::{Bus, Slot};
use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};

const APP_NAME: &str = "Factorio Bus Planner";

/// Which side(s) of the selected belt the item picker assigns to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Both,
    Left,
    Right,
}

#[derive(Clone, Copy)]
pub struct EditorState {
    pub bus: Signal<Bus>,
    /// The bus as last saved or opened, to detect unsaved changes.
    pub saved: Signal<Bus>,
    pub path: Signal<Option<PathBuf>>,
    pub selected: Signal<Option<usize>>,
    pub side: Signal<Side>,
}

impl EditorState {
    pub fn new() -> Self {
        EditorState {
            bus: Signal::new(Bus::default()),
            saved: Signal::new(Bus::default()),
            path: Signal::new(None),
            selected: Signal::new(None),
            side: Signal::new(Side::Both),
        }
    }

    pub fn is_dirty(&self) -> bool {
        *self.bus.read() != *self.saved.read()
    }

    pub fn new_bus(&mut self) {
        if self.confirm_discard() {
            self.reset(Bus::default(), None);
        }
    }

    pub fn open(&mut self) {
        if !self.confirm_discard() {
            return;
        }
        let Some(path) = file_dialog().pick_file() else {
            return;
        };
        match Bus::load(&path) {
            Ok(bus) => self.reset(bus, Some(path)),
            Err(err) => error_dialog(&format!("Could not open {}:\n{err}", path.display())),
        }
    }

    pub fn save(&mut self) {
        let path = self.path.read().clone();
        match path {
            Some(path) => self.save_to(path),
            None => self.save_as(),
        }
    }

    pub fn save_as(&mut self) {
        if let Some(path) = file_dialog().set_file_name("bus.json").save_file() {
            self.save_to(path);
        }
    }

    fn save_to(&mut self, path: PathBuf) {
        let bus = self.bus.read().clone();
        match bus.save(&path) {
            Ok(()) => {
                self.saved.set(bus);
                self.path.set(Some(path));
            }
            Err(err) => error_dialog(&format!("Could not save {}:\n{err}", path.display())),
        }
    }

    fn reset(&mut self, bus: Bus, path: Option<PathBuf>) {
        self.saved.set(bus.clone());
        self.bus.set(bus);
        self.path.set(path);
        self.selected.set(None);
    }

    fn confirm_discard(&self) -> bool {
        !self.is_dirty()
            || MessageDialog::new()
                .set_level(MessageLevel::Warning)
                .set_title(APP_NAME)
                .set_description("Discard unsaved changes to the current bus?")
                .set_buttons(MessageButtons::YesNo)
                .show()
                == MessageDialogResult::Yes
    }

    pub fn update_selected(&mut self, update: impl FnOnce(&mut Slot)) {
        let Some(index) = (self.selected)() else {
            return;
        };
        if let Some(slot) = self.bus.write().slots.get_mut(index) {
            update(slot);
        }
    }

    /// Puts an item or fluid on the selected slot, on the side(s) chosen for belts.
    pub fn assign(&mut self, name: String) {
        let side = (self.side)();
        self.update_selected(|slot| match slot {
            Slot::Belt { left, right, .. } => match side {
                Side::Both => {
                    *left = Some(name.clone());
                    *right = Some(name);
                }
                Side::Left => *left = Some(name),
                Side::Right => *right = Some(name),
            },
            Slot::Pipe { fluid, .. } => *fluid = Some(name),
            Slot::Empty => {}
        });
    }

    /// Inserts an empty belt at `index` and selects it.
    pub fn insert_at(&mut self, index: usize) {
        self.bus.write().slots.insert(index, Slot::empty_belt());
        self.selected.set(Some(index));
    }

    pub fn remove_selected(&mut self) {
        let Some(index) = (self.selected)() else {
            return;
        };
        let len = {
            let mut bus = self.bus.write();
            bus.slots.remove(index);
            bus.slots.len()
        };
        self.selected
            .set((len > 0).then(|| index.min(len - 1)));
    }

    pub fn move_selected(&mut self, offset: isize) {
        let Some(index) = (self.selected)() else {
            return;
        };
        let target = index.saturating_add_signed(offset);
        if target == index || target >= self.bus.read().slots.len() {
            return;
        }
        self.bus.write().slots.swap(index, target);
        self.selected.set(Some(target));
    }
}

fn file_dialog() -> FileDialog {
    FileDialog::new().add_filter("Bus layout", &["json"])
}

fn error_dialog(message: &str) {
    MessageDialog::new()
        .set_level(MessageLevel::Error)
        .set_title(APP_NAME)
        .set_description(message)
        .show();
}
