# Factorio Bus Planner

A tool to visualise main buses in Factorio 2.0: which item is on which lane, and which direction each lane flows.
Built for heavily modded games (primarily Nullius); visualisation only, no throughput/recipe calculations.

## Layout

Cargo workspace (edition 2024):

- `common/` (`fbp-common`): data model shared by all crates (`Catalog`, `Item`, `Group`, `Subgroup`) and its load/save.
- `cli/` (`fbp-extract`): extracts the item catalog and icons from the user's Factorio install into a local folder.
- `ui/` (planned): Dioxus desktop app that reads the extracted folder.

## Commands

```
cargo build
cargo test
cargo run -p fbp-extract -- --out data          # run Factorio dumps and build data/
cargo run -p fbp-extract -- --script-output <dir>  # reuse existing dumps, don't launch Factorio
```

## Extracted data format

`<out>/catalog.json` (serialized `fbp_common::Catalog`) plus `<out>/icons/{item,fluid,item-group}/<name>.png`.
Icon paths in the catalog are relative to `<out>`. `data/` is the default output and is git-ignored.

## How extraction works

We never parse mod Lua: Factorio does it for us with three dump flags, each producing files in `<write-data>/script-output/`:

- `--dump-data` -> `data-raw-dump.json` (final `data.raw`, ~40 MB with Nullius)
- `--dump-icon-sprites` -> pre-composited PNGs per prototype type (`item/`, `fluid/`, `item-group/`, ...)
- `--dump-prototype-locale` -> `<type>-locale.json` with a `names` map (uses the game's configured language)

Gotchas, verified on Factorio 2.0.77 (Steam):

- Only one dump flag is honoured per launch, so Factorio is launched three times.
- The Steam build relaunches itself through Steam and exits immediately unless `SteamAppId=427520` is set in the env.
- The write-data dir (where `script-output/` lives) is parsed from the `Write data path:` log line on stdout,
  which handles Steam, standalone and portable installs uniformly.
- Factorio cannot dump while another instance is running (lock file in the write-data dir).
- Every item subtype (`tool`, `module`, `capsule`, ...) is its own `data.raw` section; see `ITEM_TYPES` in the CLI.
- Mods hide prototypes with `hidden: true` (Nullius hides most vanilla items); `parameter: true` marks
  blueprint parameter placeholders. Both are filtered out.
- Empty Lua tables are dumped as `{}` even where an array is expected; deserialize only the fields we need.
