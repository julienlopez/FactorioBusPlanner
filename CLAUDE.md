# Factorio Bus Planner

A tool to visualise main buses in Factorio 2.0: which item is on which lane, and which direction each lane flows.
Built for heavily modded games (primarily Nullius); visualisation only, no throughput/recipe calculations.

## Layout

Cargo workspace (edition 2024):

- `common/` (`fbp-common`): data model shared by all crates: the extracted `Catalog` (`Item`, `Group`, `Subgroup`)
  and the `Bus` layout (`Slot`: empty / two-sided belt / pipe, each with a `Direction`), with their load/save.
- `cli/` (`fbp-extract`): extracts the item catalog and icons from the user's Factorio install into a local folder.
- `ui/` (`fbp-ui`): Dioxus 0.7 desktop app to edit bus layouts (see `ui/AGENTS.md` for the Dioxus 0.7 API).

## Commands

```
cargo build
cargo test
cargo run -p fbp-extract -- --out data          # run Factorio dumps and build data/
cargo run -p fbp-extract -- --script-output <dir>  # reuse existing dumps, don't launch Factorio
cd ui && dx serve                               # run the UI with hot reload
```

Run the UI through `dx` (`dx serve` / `dx build`): a plain `cargo run -p fbp-ui` doesn't bundle `asset!()` files, so
the CSS is missing.

## UI notes

- Desktop only. The data folder is the first CLI argument, or `data/` found by walking up from the cwd / exe dir,
  or picked with a folder dialog. It's loaded once in `main` and provided as an `Arc<Data>` context.
- Icons are served to the webview by a custom asset handler at `/fbp-data/<catalog icon path>`.
- Editor state is `EditorState` (a bundle of signals) in a context; unsaved changes = `bus != saved`.
- Bus files are plain JSON (`Bus`), opened/saved with native `rfd` dialogs.
- Belt `left`/`right` are screen sides as drawn, not relative to belt direction.
- To drive the running app for testing, launch it with
  `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` and use the Chrome DevTools Protocol.

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
