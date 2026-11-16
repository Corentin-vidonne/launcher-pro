//! Jeux autonomes (sans launcher connu) : on parcourt « Programmes et fonctionnalités » et on ne
//! retient que les dossiers qui contiennent la signature d'un moteur de jeu.

use std::fs;
use std::path::{Path, PathBuf};

use super::{is_under_steam, Ctx, Detected};
use crate::model::{Game, Launch};
use crate::util;

/// Fichiers/dossiers qui trahissent un jeu (Unity, Unreal, Godot, Ren'Py, RPG Maker, GameMaker…).
fn looks_like_game(dir: &Path) -> bool {
    const FILES: &[&str] = &[
        "UnityPlayer.dll",
        "GameAssembly.dll",
        "steam_api64.dll",
        "steam_api.dll",
        "EOSSDK-Win64-Shipping.dll",
        "data.win",
        "Game.rgss3a",
    ];
    if FILES.iter().any(|f| dir.join(f).is_file()) {
        return true;
    }
    dir.join("Engine").join("Binaries").is_dir() || (dir.join("renpy").is_dir() && dir.join("game").is_dir())
}

fn is_helper_exe(name: &str) -> bool {
    let n = name.to_lowercase();
    ["unins", "crash", "setup", "redist", "vc_", "dxsetup", "update", "report", "helper", "install"]
        .iter()
        .any(|bad| n.contains(bad))
}

fn main_exe(dir: &Path, icon: Option<&PathBuf>) -> Option<PathBuf> {
    let is_exe = |p: &Path| p.extension().map(|e| e.eq_ignore_ascii_case("exe")).unwrap_or(false);
    if let Some(icon) = icon {
        let name = icon.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if is_exe(icon) && icon.is_file() && !is_helper_exe(&name) {
            return Some(icon.clone());
        }
    }
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| is_exe(p) && !is_helper_exe(&p.file_name().unwrap_or_default().to_string_lossy()))
        .max_by_key(|p| fs::metadata(p).map(|m| m.len()).unwrap_or(0))
}

pub fn scan(ctx: &Ctx, claimed: &[String]) -> Detected {
    let mut d = Detected::new("other", "Autres jeux");
    for entry in &ctx.uninstall {
        if entry.key.starts_with("Steam App ") {
            continue;
        }
        let Some(dir) = entry.location.as_ref().filter(|p| p.is_dir()) else { continue };
        let key = util::path_key(dir);
        // Dossier déjà attribué à une source (ou qui la contient / y est contenu).
        let taken = claimed.iter().any(|c| {
            key == *c || key.starts_with(&format!("{c}\\")) || c.starts_with(&format!("{key}\\"))
        });
        if taken || is_under_steam(dir) || !looks_like_game(dir) {
            continue;
        }
        let Some(exe) = main_exe(dir, entry.icon.as_ref()) else { continue };
        let mut g = Game::new("other", util::norm_title(&entry.name).replace(' ', "-"), entry.name.clone());
        g.install_dir = Some(util::path_string(dir));
        g.size_bytes = entry.size_kb.map(|kb| kb * 1024);
        g.launch = Launch::Exe {
            path: util::path_string(&exe),
            args: None,
            cwd: Some(util::path_string(dir)),
        };
        g.art.icon_file = Some(util::path_string(&exe));
        d.games.push(g);
    }
    d
}
