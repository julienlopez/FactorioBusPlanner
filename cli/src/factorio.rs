//! Locating and launching Factorio to produce its data dumps.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};

use anyhow::{Context, Result, bail};

/// Factorio's Steam app id. Without it in the env, the Steam build relaunches itself
/// through Steam and exits immediately, detaching from us before the dump is written.
const STEAM_APP_ID: &str = "427520";

#[derive(Debug, Clone, Copy)]
pub enum Dump {
    Data,
    IconSprites,
    PrototypeLocale,
}

impl Dump {
    pub const ALL: [Dump; 3] = [Dump::Data, Dump::IconSprites, Dump::PrototypeLocale];

    fn flag(self) -> &'static str {
        match self {
            Dump::Data => "--dump-data",
            Dump::IconSprites => "--dump-icon-sprites",
            Dump::PrototypeLocale => "--dump-prototype-locale",
        }
    }

    /// A file in script-output that this dump always rewrites, used to detect a silently failed dump.
    fn marker(self) -> Option<&'static str> {
        match self {
            Dump::Data => Some("data-raw-dump.json"),
            Dump::IconSprites => None,
            Dump::PrototypeLocale => Some("item-locale.json"),
        }
    }
}

pub struct Factorio {
    pub exe: PathBuf,
    pub mod_directory: Option<PathBuf>,
}

impl Factorio {
    /// Runs one dump and returns the script-output directory it was written to.
    ///
    /// Factorio only honours a single dump flag per launch, so each dump is a separate run.
    pub fn dump(&self, dump: Dump) -> Result<PathBuf> {
        let start = SystemTime::now();
        let mut cmd = Command::new(&self.exe);
        cmd.arg(dump.flag()).env("SteamAppId", STEAM_APP_ID);
        if let Some(dir) = &self.mod_directory {
            cmd.arg("--mod-directory").arg(dir);
        }
        let output = cmd
            .output()
            .with_context(|| format!("failed to launch {}", self.exe.display()))?;
        let log = String::from_utf8_lossy(&output.stdout);
        if !output.status.success() {
            bail!(
                "Factorio {} failed ({}). Is the game already running?\n{}",
                dump.flag(),
                output.status,
                tail(&log, 20)
            );
        }

        let script_output = parse_write_data_path(&log)
            .context("could not find 'Write data path' in Factorio's output")?
            .join("script-output");
        if let Some(marker) = dump.marker() {
            let path = script_output.join(marker);
            let modified = fs::metadata(&path).and_then(|m| m.modified());
            if !modified.is_ok_and(|m| m >= start) {
                bail!(
                    "Factorio {} did not write {}\n{}",
                    dump.flag(),
                    path.display(),
                    tail(&log, 20)
                );
            }
        }
        Ok(script_output)
    }
}

/// Finds the Factorio executable, preferring the one recorded in Factorio's last log
/// so that installs in non-default locations are found.
pub fn find_exe() -> Option<PathBuf> {
    let from_log = default_write_dir()
        .and_then(|dir| fs::read_to_string(dir.join("factorio-current.log")).ok())
        .and_then(|log| parse_program_path(&log));
    from_log
        .into_iter()
        .chain(default_exe_locations())
        .find(|path| path.is_file())
}

/// Factorio's write-data directory for standard (non-portable) installs.
pub fn default_write_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        env::var_os("APPDATA").map(|dir| Path::new(&dir).join("Factorio"))
    } else if cfg!(target_os = "macos") {
        home().map(|home| home.join("Library/Application Support/factorio"))
    } else {
        home().map(|home| home.join(".factorio"))
    }
}

fn default_exe_locations() -> Vec<PathBuf> {
    if cfg!(windows) {
        vec![
            r"C:\Program Files (x86)\Steam\steamapps\common\Factorio\bin\x64\factorio.exe".into(),
            r"C:\Program Files\Factorio\bin\x64\factorio.exe".into(),
        ]
    } else if cfg!(target_os = "macos") {
        home()
            .map(|home| {
                home.join("Library/Application Support/Steam/steamapps/common/Factorio/factorio.app/Contents/MacOS/factorio")
            })
            .into_iter()
            .collect()
    } else {
        home()
            .map(|home| home.join(".steam/steam/steamapps/common/Factorio/bin/x64/factorio"))
            .into_iter()
            .collect()
    }
}

fn home() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

/// Parses `   0.000 Write data path: C:/Users/x/AppData/Roaming/Factorio [1666075/1906638MB]`.
fn parse_write_data_path(log: &str) -> Option<PathBuf> {
    let line = log.lines().find_map(|l| l.split_once("Write data path: "))?.1;
    let path = line.rsplit_once(" [").map_or(line, |(path, _)| path);
    Some(PathBuf::from(path.trim()))
}

/// Parses `   0.000 Program arguments: "D:\Steam\...\Factorio.exe" "--flag"`.
fn parse_program_path(log: &str) -> Option<PathBuf> {
    let args = log.lines().find_map(|l| l.split_once("Program arguments: "))?.1;
    let path = args.strip_prefix('"')?.split('"').next()?;
    Some(PathBuf::from(path))
}

fn tail(text: &str, lines: usize) -> String {
    let all: Vec<_> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_write_data_path() {
        let log = "   0.000 Read data path: D:/Steam/Factorio/data\n   0.000 Write data path: C:/Users/x/AppData/Roaming/Factorio [1666075/1906638MB]\n";
        assert_eq!(
            parse_write_data_path(log),
            Some(PathBuf::from("C:/Users/x/AppData/Roaming/Factorio"))
        );
    }

    #[test]
    fn parses_program_path() {
        let log = r#"   0.000 Program arguments: "D:\Steam\Factorio\bin\x64\Factorio.exe" "--dump-data" "#;
        assert_eq!(
            parse_program_path(log),
            Some(PathBuf::from(r"D:\Steam\Factorio\bin\x64\Factorio.exe"))
        );
    }
}
