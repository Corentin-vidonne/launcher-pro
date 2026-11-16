//! EA app : les jeux s'enregistrent dans « Programmes » avec le désinstalleur commun EAInstaller.

use std::fs;
use std::path::{Path, PathBuf};

use super::{is_under_steam, Ctx, Detected};
use crate::model::{Game, Launch};
use crate::util;

pub fn scan(ctx: &Ctx) -> Vec<Detected> {
    let key = r"SOFTWARE\Electronic Arts\EA Desktop";
    let launcher = ["LauncherAppPath", "DesktopAppPath", "ClientPath"]
        .iter()
        .find_map(|name| util::hklm_string(key, name))
        .map(PathBuf::from)
        .filter(|p| p.is_file());

    let mut d = Detected::new("ea", "EA app");
    if let Some(exe) = &launcher {
        // L'icône la plus parlante est celle de l'application, pas du petit lanceur.
        let desktop = util::hklm_string(key, "DesktopAppPath").map(PathBuf::from).filter(|p| p.is_file());
        d = d.with_launcher_exe(exe);
        if let Some(desktop) = desktop {
            d.icon_file = Some(desktop);
        }
    }

    for entry in &ctx.uninstall {
        if !entry.uninstall.to_lowercase().contains("eainstaller") {
            continue;
        }
        let Some(dir) = entry.location.as_ref().filter(|p| p.is_dir()) else { continue };
        if is_under_steam(dir) {
            continue; // version Steam : déjà listée côté Steam
        }
        let mut g = Game::new("ea", util::norm_title(&entry.name).replace(' ', "-"), entry.name.clone());
        g.install_dir = Some(util::path_string(dir));
        g.size_bytes = entry.size_kb.map(|kb| kb * 1024);

        let exe = entry
            .icon
            .clone()
            .filter(|p| is_exe(p))
            .or_else(|| exe_from_installer_data(dir));
        if let Some(exe) = exe {
            // Lancer l'exécutable suffit : le jeu réveille EA app pour l'authentification.
            g.launch = Launch::Exe {
                path: util::path_string(&exe),
                args: None,
                cwd: Some(util::path_string(dir)),
            };
            g.art.icon_file = Some(util::path_string(&exe));
        } else if let Some(l) = &launcher {
            g.launch = Launch::exe(util::path_string(l));
        }
        d.games.push(g);
    }

    if d.installed || !d.games.is_empty() {
        vec![d]
    } else {
        Vec::new()
    }
}

fn is_exe(p: &Path) -> bool {
    p.extension().map(|e| e.eq_ignore_ascii_case("exe")).unwrap_or(false) && p.is_file()
}

/// `__Installer\installerdata.xml` : `<filePath>[HKEY…\Install Dir]Jeu.exe</filePath>`.
fn exe_from_installer_data(dir: &Path) -> Option<PathBuf> {
    let xml = fs::read_to_string(dir.join("__Installer").join("installerdata.xml")).ok()?;
    let raw = util::xml_tag(&xml, "filePath")?;
    let relative = raw.rsplit(']').next()?.trim();
    let candidate = dir.join(relative.replace('/', "\\"));
    is_exe(&candidate).then_some(candidate)
}
