//! Turns Factorio's script-output dumps into a `Catalog` and an icon folder.

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

use anyhow::{Context, Result};
use fbp_common::{Catalog, Group, Item, ItemKind, Subgroup};
use serde::Deserialize;
use serde_json::value::RawValue;

/// Every `data.raw` section holding item prototypes (all subtypes of ItemPrototype in 2.0).
const ITEM_TYPES: &[&str] = &[
    "item",
    "ammo",
    "armor",
    "blueprint",
    "blueprint-book",
    "capsule",
    "copy-paste-tool",
    "deconstruction-item",
    "gun",
    "item-with-entity-data",
    "item-with-inventory",
    "item-with-label",
    "item-with-tags",
    "module",
    "rail-planner",
    "repair-tool",
    "selection-tool",
    "space-platform-starter-pack",
    "spidertron-remote",
    "tool",
    "upgrade-item",
];

/// The few prototype fields we care about; everything else in the dump is ignored.
#[derive(Deserialize)]
struct RawPrototype {
    name: String,
    subgroup: Option<String>,
    group: Option<String>,
    #[serde(default)]
    order: String,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    parameter: bool,
}

#[derive(Deserialize)]
struct LocaleDump {
    names: HashMap<String, String>,
}

/// Builds the catalog from `script_output` and copies the needed icons into `out/icons`.
pub fn extract(script_output: &Path, out: &Path) -> Result<Catalog> {
    let dump_path = script_output.join("data-raw-dump.json");
    let dump_text = fs::read_to_string(&dump_path)
        .with_context(|| format!("failed to read {}", dump_path.display()))?;
    let data_raw: HashMap<String, &RawValue> =
        serde_json::from_str(&dump_text).context("failed to parse data-raw-dump.json")?;
    let section = |type_name: &str| -> Result<Vec<RawPrototype>> {
        let Some(raw) = data_raw.get(type_name) else {
            return Ok(Vec::new());
        };
        let protos: HashMap<String, RawPrototype> = serde_json::from_str(raw.get())
            .with_context(|| format!("failed to parse data.raw[\"{type_name}\"]"))?;
        Ok(protos.into_values().collect())
    };

    let item_names = load_locale(script_output, "item")?;
    let fluid_names = load_locale(script_output, "fluid")?;
    let group_names = load_locale(script_output, "item-group")?;

    let mut items = Vec::new();
    for type_name in ITEM_TYPES {
        for proto in section(type_name)? {
            items.extend(to_item(proto, ItemKind::Item, "other", &item_names));
        }
    }
    for proto in section("fluid")? {
        items.extend(to_item(proto, ItemKind::Fluid, "fluid", &fluid_names));
    }

    let mut subgroups: Vec<Subgroup> = section("item-subgroup")?
        .into_iter()
        .map(|proto| Subgroup {
            group: proto.group.unwrap_or_default(),
            name: proto.name,
            order: proto.order,
        })
        .collect();
    let mut groups: Vec<Group> = section("item-group")?
        .into_iter()
        .filter(|proto| !proto.hidden)
        .map(|proto| Group {
            localised_name: localise(&group_names, &proto.name),
            name: proto.name,
            order: proto.order,
            icon: None,
        })
        .collect();

    // Keep only the groups and subgroups that end up containing a visible item.
    let known_subgroups: HashSet<&str> = subgroups.iter().map(|s| s.name.as_str()).collect();
    items.retain(|item| {
        let known = known_subgroups.contains(item.subgroup.as_str());
        if !known {
            eprintln!("warning: skipping {} (unknown subgroup {})", item.name, item.subgroup);
        }
        known
    });
    let used_subgroups: HashSet<String> = items.iter().map(|i| i.subgroup.clone()).collect();
    subgroups.retain(|s| used_subgroups.contains(&s.name));
    let used_groups: HashSet<&str> = subgroups.iter().map(|s| s.group.as_str()).collect();
    groups.retain(|g| used_groups.contains(g.name.as_str()));
    let kept_groups: HashSet<&str> = groups.iter().map(|g| g.name.as_str()).collect();
    subgroups.retain(|s| kept_groups.contains(s.group.as_str()));
    let kept_subgroups: HashSet<&str> = subgroups.iter().map(|s| s.name.as_str()).collect();
    items.retain(|i| kept_subgroups.contains(i.subgroup.as_str()));

    // Sort like the in-game crafting menu: by group, then subgroup, then item order strings.
    groups.sort_by(|a, b| (&a.order, &a.name).cmp(&(&b.order, &b.name)));
    let group_rank: HashMap<&str, usize> =
        groups.iter().enumerate().map(|(i, g)| (g.name.as_str(), i)).collect();
    subgroups.sort_by(|a, b| {
        (group_rank[a.group.as_str()], &a.order, &a.name)
            .cmp(&(group_rank[b.group.as_str()], &b.order, &b.name))
    });
    let subgroup_rank: HashMap<&str, usize> =
        subgroups.iter().enumerate().map(|(i, s)| (s.name.as_str(), i)).collect();
    items.sort_by(|a, b| {
        (subgroup_rank[a.subgroup.as_str()], &a.order, &a.name)
            .cmp(&(subgroup_rank[b.subgroup.as_str()], &b.order, &b.name))
    });

    copy_icons(script_output, out, &mut groups, &mut items)?;
    Ok(Catalog {
        groups,
        subgroups,
        items,
    })
}

/// Converts a raw prototype, or returns `None` if it never shows up in game.
fn to_item(
    proto: RawPrototype,
    kind: ItemKind,
    default_subgroup: &str,
    names: &HashMap<String, String>,
) -> Option<Item> {
    if proto.hidden || proto.parameter {
        return None;
    }
    Some(Item {
        localised_name: localise(names, &proto.name),
        subgroup: proto.subgroup.unwrap_or_else(|| default_subgroup.to_owned()),
        name: proto.name,
        kind,
        order: proto.order,
        icon: None,
    })
}

fn load_locale(script_output: &Path, type_name: &str) -> Result<HashMap<String, String>> {
    let path = script_output.join(format!("{type_name}-locale.json"));
    let text =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let dump: LocaleDump = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(dump.names)
}

fn localise(names: &HashMap<String, String>, name: &str) -> String {
    names.get(name).cloned().unwrap_or_else(|| name.to_owned())
}

/// Replaces `out/icons` with the icons of the catalog entries and records their relative paths.
fn copy_icons(
    script_output: &Path,
    out: &Path,
    groups: &mut [Group],
    items: &mut [Item],
) -> Result<()> {
    let icons_dir = out.join("icons");
    if icons_dir.exists() {
        fs::remove_dir_all(&icons_dir)
            .with_context(|| format!("failed to clear {}", icons_dir.display()))?;
    }

    let copy =|dir: &str, name: &str| -> Result<Option<String>> {
        let source = script_output.join(dir).join(format!("{name}.png"));
        if !source.is_file() {
            eprintln!("warning: no icon for {dir} {name}");
            return Ok(None);
        }
        let relative = format!("icons/{dir}/{name}.png");
        let target = out.join(&relative);
        fs::create_dir_all(target.parent().unwrap())?;
        fs::copy(&source, &target)
            .with_context(|| format!("failed to copy {}", source.display()))?;
        Ok(Some(relative))
    };

    for group in groups {
        group.icon = copy("item-group", &group.name)?;
    }
    for item in items {
        let dir = match item.kind {
            ItemKind::Item => "item",
            ItemKind::Fluid => "fluid",
        };
        item.icon = copy(dir, &item.name)?;
    }
    Ok(())
}
