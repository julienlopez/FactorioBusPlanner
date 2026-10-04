//! The extracted game data (catalog + icons) produced by `fbp-extract`.

use std::{
    collections::HashMap,
    env, io,
    path::{Path, PathBuf},
};

use fbp_common::{CATALOG_FILE, Catalog, Item, ItemKind};

/// URL prefix under which icons from the data folder are served to the webview.
pub const ICON_ROUTE: &str = "fbp-data";

pub struct Data {
    pub dir: PathBuf,
    pub catalog: Catalog,
    items: HashMap<(ItemKind, String), usize>,
    subgroup_groups: HashMap<String, String>,
}

impl Data {
    pub fn load(dir: PathBuf) -> io::Result<Self> {
        let catalog = Catalog::load(&dir)?;
        let items = catalog
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| ((item.kind, item.name.clone()), i))
            .collect();
        let subgroup_groups = catalog
            .subgroups
            .iter()
            .map(|s| (s.name.clone(), s.group.clone()))
            .collect();
        Ok(Data {
            dir,
            catalog,
            items,
            subgroup_groups,
        })
    }

    pub fn item(&self, kind: ItemKind, name: &str) -> Option<&Item> {
        let index = self.items.get(&(kind, name.to_owned()))?;
        Some(&self.catalog.items[*index])
    }

    /// The in-game name of an item, or its internal name if it isn't in the catalog.
    pub fn display_name(&self, kind: ItemKind, name: &str) -> String {
        self.item(kind, name)
            .map_or_else(|| name.to_owned(), |item| item.localised_name.clone())
    }

    pub fn group_of(&self, item: &Item) -> &str {
        self.subgroup_groups
            .get(&item.subgroup)
            .map_or("", String::as_str)
    }
}

/// URL of an icon path from the catalog.
pub fn icon_url(icon: &str) -> String {
    format!("/{ICON_ROUTE}/{icon}")
}

/// Looks for `data/catalog.json` in the working directory, the executable's directory,
/// and their ancestors, so the app works from `cargo run`, `dx serve` and a built binary.
pub fn find_data_dir() -> Option<PathBuf> {
    let starts = [
        env::current_dir().ok(),
        env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf)),
    ];
    starts
        .into_iter()
        .flatten()
        .flat_map(|start| {
            start
                .ancestors()
                .map(|dir| dir.join("data"))
                .collect::<Vec<_>>()
        })
        .find(|dir| dir.join(CATALOG_FILE).is_file())
}
