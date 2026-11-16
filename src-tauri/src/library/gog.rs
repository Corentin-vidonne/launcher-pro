//! GOG : chaque jeu installé (avec ou sans Galaxy) a sa clé `GOG.com\Games\<id>`.

use std::path::PathBuf;

use super::{Ctx, Detected};
use crate::model::{Game, Launch};
use crate::util;

pub fn scan(_ctx: &Ctx) -> Vec<Detected> {
    let client = util::hklm_string(r"SOFTWARE\GOG.com\GalaxyClient\paths", "client")
        .map(|dir| PathBuf::from(dir).join("GalaxyClient.exe"))
        .filter(|p| p.is_file());
    let mut d = Detected::new("gog", "GOG Galaxy");
    if let Some(exe) = &client {
        d = d.with_launcher_exe(exe);
    }

    for games in util::hklm_views(r"SOFTWARE\GOG.com\Games") {
        for id in games.enum_keys().flatten() {
            let Ok(key) = games.open_subkey(&id) else { continue };
            // Les DLC déclarent le jeu dont ils dépendent.
            if util::reg_string(&key, "dependsOn").is_some() {
                continue;
            }
            let get = |names: &[&str]| names.iter().find_map(|n| util::reg_string(&key, n));
            let Some(name) = get(&["gameName", "GAMENAME"]) else { continue };
            let Some(dir) = get(&["path", "PATH"]).map(PathBuf::from).filter(|p| p.is_dir()) else { continue };
            if d.games.iter().any(|g| g.id == format!("gog:{id}")) {
                continue;
            }
            let mut g = Game::new("gog", &id, name);
            g.install_dir = Some(util::path_string(&dir));
            if let Some(exe) = get(&["exe", "EXE"]).map(PathBuf::from).filter(|p| p.is_file()) {
                g.launch = Launch::Exe {
                    path: util::path_string(&exe),
                    args: None,
                    cwd: get(&["workingDir", "WORKINGDIR"]).or_else(|| Some(util::path_string(&dir))),
                };
                g.art.icon_file = Some(util::path_string(&exe));
            } else if let Some(client) = &client {
                g.launch = Launch::exe_args(
                    util::path_string(client),
                    format!("/command=runGame /gameId={id}"),
                );
            }
            d.games.push(g);
        }
    }
    if d.installed || !d.games.is_empty() {
        vec![d]
    } else {
        Vec::new()
    }
}
