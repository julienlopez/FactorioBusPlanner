mod bus_view;
mod data;
mod editor;
mod state;

use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use dioxus::{
    desktop::{
        Config, LogicalSize, WindowBuilder, use_asset_handler,
        wry::http::{Response, StatusCode},
    },
    prelude::*,
};
use fbp_common::CATALOG_FILE;
use rfd::{MessageDialog, MessageLevel};

use crate::{
    bus_view::BusView,
    data::{Data, ICON_ROUTE},
    editor::SlotEditor,
    state::EditorState,
};

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    let data = match load_data() {
        Ok(data) => data,
        Err(message) => {
            MessageDialog::new()
                .set_level(MessageLevel::Error)
                .set_title("Factorio Bus Planner")
                .set_description(message)
                .show();
            return;
        }
    };

    let window = WindowBuilder::new()
        .with_title("Factorio Bus Planner")
        .with_inner_size(LogicalSize::new(1400.0, 900.0));
    dioxus::LaunchBuilder::desktop()
        .with_cfg(Config::new().with_window(window).with_menu(None))
        .with_context(Arc::new(data))
        .launch(App);
}

/// Loads the data folder given as first argument, found next to the app, or picked by the user.
fn load_data() -> Result<Data, String> {
    let dir = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .or_else(data::find_data_dir)
        .or_else(|| {
            rfd::FileDialog::new()
                .set_title("Select the data folder created by fbp-extract")
                .pick_folder()
        })
        .ok_or("No data folder selected.")?;
    Data::load(dir.clone()).map_err(|err| {
        format!(
            "Could not load {}: {err}\n\nRun `cargo run -p fbp-extract` to extract the game data first.",
            dir.join(CATALOG_FILE).display()
        )
    })
}

#[component]
fn App() -> Element {
    let data = use_context::<Arc<Data>>();
    use_icon_server(data.dir.clone());
    use_context_provider(EditorState::new);

    rsx! {
        document::Stylesheet { href: MAIN_CSS }
        div { class: "app",
            Toolbar {}
            div { class: "workspace",
                BusView {}
                aside { class: "sidebar", SlotEditor {} }
            }
        }
    }
}

/// Serves `/fbp-data/<path>` from the data folder, so the webview can display icons.
fn use_icon_server(dir: PathBuf) {
    use_asset_handler(ICON_ROUTE, move |request, responder| {
        let prefix = format!("/{ICON_ROUTE}/");
        let icon = request
            .uri()
            .path()
            .strip_prefix(&prefix)
            .and_then(|relative| read_icon(&dir, relative));
        let response = match icon {
            Some(bytes) => Response::builder()
                .header("Content-Type", "image/png")
                .body(bytes),
            None => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Vec::new()),
        };
        responder.respond(response.expect("static response parts are valid"));
    });
}

fn read_icon(dir: &Path, relative: &str) -> Option<Vec<u8>> {
    // Never serve anything outside the data folder.
    if relative.split('/').any(|part| part == "..") {
        return None;
    }
    fs::read(dir.join(relative)).ok()
}

#[component]
fn Toolbar() -> Element {
    let mut state = use_context::<EditorState>();
    let file_name = state
        .path
        .read()
        .as_ref()
        .and_then(|path| path.file_name())
        .map_or_else(|| "Untitled".to_owned(), |name| name.to_string_lossy().into_owned());
    let dirty = state.is_dirty();

    rsx! {
        header { class: "toolbar",
            button { onclick: move |_| state.new_bus(), "New" }
            button { onclick: move |_| state.open(), "Open…" }
            button { onclick: move |_| state.save(), "Save" }
            button { onclick: move |_| state.save_as(), "Save as…" }
            span { class: "separator" }
            button { onclick: move |_| state.bus.write().push_group(), "Add group" }
            span { class: "file-name",
                "{file_name}"
                if dirty { " •" }
            }
        }
    }
}
