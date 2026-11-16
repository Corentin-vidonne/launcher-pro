//! Wargaming Game Center (World of Tanks, World of Warships…) et sa variante Lesta.

use std::fs;
use std::path::{Path, PathBuf};

use super::{uninstall_name_for_dir, Ctx, Detected};
use crate::model::{Game, Launch};
use crate::util;

fn name_from_id(id: &str) -> Option<&'static str> {
    Some(match id.split('.').next()?.to_ascii_uppercase().as_str() {
        "WOT" => "World of Tanks",
        "WOWS" => "World of Warships",
        "WOWP" => "World of Warplanes",
        "WOTB" => "World of Tanks Blitz",
        _ => return None,
    })
}

pub fn scan(ctx: &Ctx) -> Vec<Detected> {
    let centers = [
        (util::program_data().join("Wargaming.net").join("GameCenter"), "wgc.exe"),
        (util::program_data().join("Lesta").join("GameCenter"), "lgc.exe"),
    ];
    let mut d = Detected::new("wargaming", "Wargaming");
    for (center, exe_name) in centers {
        if !center.is_dir() {
            continue;
        }
        let launcher = center.join(exe_name);
        if launcher.is_file() && !d.installed {
            d = d.with_launcher_exe(&launcher);
        }
        // apps\<id>\<empreinte> : petit fichier texte contenant le dossier du jeu.
        for app in fs::read_dir(center.join("apps")).into_iter().flatten().flatten() {
            for file in fs::read_dir(app.path()).into_iter().flatten().flatten() {
                let Ok(content) = fs::read_to_string(file.path()) else { continue };
                let dir = PathBuf::from(content.trim().trim_matches('\0'));
                if !dir.is_dir() {
                    continue;
                }
                let info = fs::read_to_string(dir.join("game_info.xml")).unwrap_or_default();
                let game_id = util::xml_tag(&info, "id").unwrap_or("").to_string();
                let key = if game_id.is_empty() { util::path_key(&dir) } else { game_id.to_lowercase() };
                let name = uninstall_name_for_dir(ctx, &dir)
                    .or_else(|| name_from_id(&game_id).map(str::to_string))
                    .or_else(|| dir.file_name().map(|n| n.to_string_lossy().to_string()));
                let Some(name) = name else { continue };

                let mut g = Game::new("wargaming", key, name);
                g.install_dir = Some(util::path_string(&dir));
                // Le raccourci officiel passe par `wgc_api.exe --open` (connexion automatique).
                let api = dir.join("wgc_api.exe");
                if api.is_file() {
                    g.launch = Launch::Exe {
                        path: util::path_string(&api),
                        args: Some("--open".into()),
                        cwd: Some(util::path_string(&dir)),
                    };
                } else if launcher.is_file() {
                    g.launch = Launch::exe(util::path_string(&launcher));
                }
                let icon = dir.join("game_metadata").join("game.ico");
                g.art.icon_file = first_file(&[icon]).map(|p| util::path_string(&p));
                d.games.push(g);
            }
        }
    }
    if d.installed || !d.games.is_empty() {
        vec![d]
    } else {
        Vec::new()
    }
}

fn first_file(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|p| Path::new(p).is_file()).cloned()
}
