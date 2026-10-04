//! Extracts the item catalog and icons of the current Factorio mod set into a local folder.

mod extract;
mod factorio;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use fbp_common::{CATALOG_FILE, ItemKind};

use crate::factorio::{Dump, Factorio};

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Path to the Factorio executable. Auto-detected if omitted.
    #[arg(long)]
    factorio: Option<PathBuf>,

    /// Mod directory to load instead of Factorio's default one.
    #[arg(long)]
    mod_directory: Option<PathBuf>,

    /// Reuse the dumps already present in this script-output directory instead of launching Factorio.
    #[arg(long, conflicts_with_all = ["factorio", "mod_directory"])]
    script_output: Option<PathBuf>,

    /// Folder to write the catalog and icons to.
    #[arg(long, short, default_value = "data")]
    out: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let script_output = match args.script_output {
        Some(dir) => dir,
        None => {
            let exe = match args.factorio {
                Some(exe) => exe,
                None => factorio::find_exe()
                    .context("could not find Factorio, pass its executable with --factorio")?,
            };
            println!("Using {}", exe.display());
            let factorio = Factorio {
                exe,
                mod_directory: args.mod_directory,
            };
            let mut script_output = PathBuf::new();
            for dump in Dump::ALL {
                println!("Running Factorio {dump:?} dump...");
                script_output = factorio.dump(dump)?;
            }
            script_output
        }
    };

    println!("Extracting from {}", script_output.display());
    let catalog = extract::extract(&script_output, &args.out)?;
    catalog
        .save(&args.out)
        .with_context(|| format!("failed to write {}", args.out.join(CATALOG_FILE).display()))?;

    let fluids = catalog.items.iter().filter(|i| i.kind == ItemKind::Fluid).count();
    println!(
        "Wrote {} items, {} fluids in {} groups to {}",
        catalog.items.len() - fluids,
        fluids,
        catalog.groups.len(),
        args.out.display()
    );
    Ok(())
}
