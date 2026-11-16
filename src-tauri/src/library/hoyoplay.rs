//! HoYoPlay (Genshin Impact, Honkai: Star Rail, Zenless Zone Zero…) : tout est dans HKCU.

use std::path::{Path, PathBuf};

use super::{Ctx, Detected};
use crate::model::{Game, Launch};
use crate::util;

/// Versions internationale et chinoise du launcher.
const ROOTS: &[&str] = &[r"Software\Cognosphere\HYP", r"Software\miHoYo\HYP"];

fn game_name(biz: &str) -> Option<&'static str> {
    Some(match biz.split('_').next()? {
        "hk4e" => "Genshin Impact",
        "hkrpg" => "Honkai: Star Rail",
        "nap" => "Zenless Zone Zero",
        "bh3" => "Honkai Impact 3rd",
        _ => return None,
    })
}

pub fn scan(ctx: &Ctx) -> Vec<Detected> {
    let mut d = Detected::new("hoyoplay", "HoYoPlay");
    for root in ROOTS {
        let Some(hyp) = util::hkcu(root) else { continue };
        for version in hyp.enum_keys().flatten() {
            let Ok(vkey) = hyp.open_subkey(&version) else { continue };
            let launcher = util::reg_string(&vkey, "InstallPath")
                .map(|p| PathBuf::from(p).join("launcher.exe"))
                .filter(|p| p.is_file());
            if let Some(exe) = &launcher {
                if !d.installed {
                    d = d.with_launcher_exe(exe);
                }
            }
            for biz in vkey.enum_keys().flatten() {
                let Ok(gkey) = vkey.open_subkey(&biz) else { continue };
                let Some(dir) = util::reg_string(&gkey, "GameInstallPath").filter(|p| Path::new(p).is_dir()) else {
                    continue;
                };
                // Nom officiel : table interne, sinon l'entrée « Programmes » du jeu (clé `<biz>_…`).
                let name = game_name(&biz).map(str::to_string).or_else(|| {
                    ctx.uninstall
                        .iter()
                        .find(|e| e.key.starts_with(&format!("{biz}_")))
                        .map(|e| e.name.clone())
                });
                let Some(name) = name else { continue };
                let mut g = Game::new("hoyoplay", &biz, name);
                g.install_dir = Some(dir);
                if let Some(exe) = &launcher {
                    // Même commande que les raccourcis officiels : le launcher gère les mises à jour.
                    g.launch = Launch::exe_args(util::path_string(exe), format!("--game={biz}"));
                }
                g.art.icon_file = util::reg_string(&gkey, "GameIconPath").filter(|p| Path::new(p).is_file());
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
