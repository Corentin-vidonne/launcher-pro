//! Ubisoft Connect : `Ubisoft\Launcher\Installs\<id>` dans le registre.

use std::path::PathBuf;

use super::{is_under_steam, Ctx, Detected};
use crate::model::{Game, Launch};
use crate::util;

const ROOT: &str = r"SOFTWARE\Ubisoft\Launcher";

pub fn scan(ctx: &Ctx) -> Vec<Detected> {
    let client = util::hklm_string(ROOT, "InstallDir")
        .map(|dir| PathBuf::from(dir).join("UbisoftConnect.exe"))
        .filter(|p| p.is_file());
    let Some(client) = client else { return Vec::new() };
    let mut d = Detected::new("ubisoft", "Ubisoft Connect").with_launcher_exe(&client);

    for installs in util::hklm_views(&format!(r"{ROOT}\Installs")) {
        for id in installs.enum_keys().flatten() {
            let Ok(key) = installs.open_subkey(&id) else { continue };
            let Some(dir) = util::reg_string(&key, "InstallDir")
                .map(|p| PathBuf::from(p.replace('/', "\\").trim_end_matches('\\')))
                .filter(|p| p.is_dir())
            else {
                continue;
            };
            if is_under_steam(&dir) || d.games.iter().any(|g| g.id == format!("ubisoft:{id}")) {
                continue;
            }
            // Le nom lisible est dans l'entrée « Programmes » : `Uplay Install <id>`.
            let name = ctx
                .uninstall
                .iter()
                .find(|e| e.key == format!("Uplay Install {id}"))
                .map(|e| e.name.clone())
                .or_else(|| dir.file_name().map(|n| n.to_string_lossy().to_string()));
            let Some(name) = name else { continue };
            let mut g = Game::new("ubisoft", &id, name);
            g.install_dir = Some(util::path_string(&dir));
            g.launch = Launch::uri(format!("uplay://launch/{id}/0"));
            d.games.push(g);
        }
    }
    vec![d]
}
