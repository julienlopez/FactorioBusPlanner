//! Data model shared between the extractor CLI and the UI.

use std::{fs, io, path::Path};

use serde::{Deserialize, Serialize};

/// File name of the catalog inside an extracted data folder.
pub const CATALOG_FILE: &str = "catalog.json";

/// Everything that can be put on a bus, as extracted from a Factorio mod set.
///
/// All lists are sorted in the same order as the in-game crafting menu.
/// Icon paths are relative to the data folder the catalog was loaded from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    pub groups: Vec<Group>,
    pub subgroups: Vec<Subgroup>,
    pub items: Vec<Item>,
}

/// A crafting menu tab (e.g. "Logistics").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    pub localised_name: String,
    pub order: String,
    pub icon: Option<String>,
}

/// A row inside a crafting menu tab.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subgroup {
    pub name: String,
    pub group: String,
    pub order: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub name: String,
    pub kind: ItemKind,
    pub localised_name: String,
    pub subgroup: String,
    pub order: String,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    Item,
    Fluid,
}

impl Catalog {
    pub fn load(data_dir: &Path) -> io::Result<Self> {
        let text = fs::read_to_string(data_dir.join(CATALOG_FILE))?;
        Ok(serde_json::from_str(&text)?)
    }

    pub fn save(&self, data_dir: &Path) -> io::Result<()> {
        fs::create_dir_all(data_dir)?;
        let text = serde_json::to_string_pretty(self)?;
        fs::write(data_dir.join(CATALOG_FILE), text)
    }
}
