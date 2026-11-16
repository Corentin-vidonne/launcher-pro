//! Rockstar Games Launcher : une clé de registre par jeu, avec `InstallFolder`.

use std::path::PathBuf;

use super::{is_under_steam, Ctx, Detected};
use crate::model::{Game, Launch};
use crate::util;

const ROOT: &str = r"SOFTWARE\Rockstar Games";
const NOT_GAMES: &[&str] = &["Launcher", "Rockstar Games Social Club", "Steam"];

pub fn scan(_ctx: &Ctx) -> Vec<Detected> {
    let launcher = util::hklm_string(&format!(r"{ROOT}\Launcher"), "InstallFolder")
        .map(|dir| PathBuf::from(dir).join("Launcher.exe"))
        .filter(|p| p.is_file());
    let Some(launcher) = launcher else { return Vec::new() };
    let mut d = Detected::new("rockstar", "Rockstar Games").with_launcher_exe(&launcher);

    for root in util::hklm_views(ROOT) {
        for title in root.enum_keys().flatten() {
            if NOT_GAMES.contains(&title.as_str()) {
                continue;
            }
            let Ok(key) = root.open_subkey(&title) else { continue };
            let Some(dir) = util::reg_string(&key, "InstallFolder").map(PathBuf::from).filter(|p| p.is_dir()) else {
                continue; // clé résiduelle d'un jeu désinstallé
            };
            if is_under_steam(&dir) || d.games.iter().any(|g| g.name == title) {
                continue;
            }
            let mut g = Game::new("rockstar", util::norm_title(&title).replace(' ', "-"), title);
            g.install_dir = Some(util::path_string(&dir));
            g.launch = Launch::exe_args(
                util::path_string(&launcher),
                format!("-launchTitleInFolder \"{}\"", util::path_string(&dir)),
            );
            d.games.push(g);
        }
    }
    vec![d]
}
