//! Launchers « mono-éditeur » : Kuro Games (Wuthering Waves), GRYPHLINK (Arknights: Endfield),
//! Ankama (Dofus, Wakfu…). Chacun n'expose qu'une poignée de jeux, lancés via leur launcher.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{Ctx, Detected};
use crate::model::{Game, Launch};
use crate::util;

pub fn scan(ctx: &Ctx) -> Vec<Detected> {
    [kuro(ctx), gryphline(ctx), ankama(ctx)].into_iter().flatten().collect()
}

fn fixed_drives() -> Vec<PathBuf> {
    (b'C'..=b'Z')
        .map(|l| PathBuf::from(format!("{}:\\", l as char)))
        .filter(|p| p.is_dir())
        .collect()
}

/// Kuro Games : entrée « KRInstall <jeu> », launcher = `launcher.exe`.
fn kuro(ctx: &Ctx) -> Option<Detected> {
    let mut d = Detected::new("kuro", "Kuro Games");
    for entry in ctx.uninstall.iter().filter(|e| e.key.starts_with("KRInstall")) {
        let Some(launcher) = entry.icon.clone().filter(|p| p.is_file()) else { continue };
        let launcher_dir = launcher.parent()?.to_path_buf();
        if !d.installed {
            d = d.with_launcher_exe(&launcher);
        }
        // Le jeu peut vivre ailleurs que le launcher (choix à l'installation) : on regarde les
        // emplacements usuels, sur tous les disques.
        let game_folder = format!("{} Game", entry.name);
        let mut candidates = vec![launcher_dir.join(&game_folder)];
        for drive in fixed_drives() {
            candidates.push(drive.join(&entry.name).join(&game_folder));
            candidates.push(drive.join(&game_folder));
            candidates.push(drive.join("Program Files").join(&entry.name).join(&game_folder));
        }
        let game_dir = candidates.into_iter().find(|p| p.is_dir());
        let downloaded = launcher_dir.join("kr_game_cache").join("KRDownloadRecordCache.json").is_file();
        if game_dir.is_none() && !downloaded {
            continue; // launcher présent mais jeu jamais téléchargé
        }
        let mut g = Game::new("kuro", util::norm_title(&entry.name).replace(' ', "-"), entry.name.clone());
        g.install_dir = game_dir.as_deref().map(util::path_string);
        g.launch = Launch::exe(util::path_string(&launcher));
        g.art.icon_file = game_dir
            .map(|dir| dir.join(format!("{}.exe", entry.name)))
            .filter(|p| p.is_file())
            .or(Some(launcher))
            .map(|p| util::path_string(&p));
        d.games.push(g);
    }
    (d.installed || !d.games.is_empty()).then_some(d)
}

/// GRYPHLINK : les jeux sont installés dans `<launcher>\games\<Nom> Game`.
fn gryphline(ctx: &Ctx) -> Option<Detected> {
    let entry = ctx.uninstall.iter().find(|e| e.name == "GRYPHLINK")?;
    let dir = entry.location.clone()?;
    let launcher = dir.join("Launcher.exe");
    if !launcher.is_file() {
        return None;
    }
    let mut d = Detected::new("gryphline", "GRYPHLINK").with_launcher_exe(&launcher);
    for game in fs::read_dir(dir.join("games")).into_iter().flatten().flatten() {
        let path = game.path();
        let Some(exe) = root_exe(&path) else { continue }; // dossier vide = jeu non installé
        let folder = game.file_name().to_string_lossy().to_string();
        let name = match folder.to_lowercase().as_str() {
            "endfield game" => "Arknights: Endfield".to_string(),
            _ => folder.trim_end_matches(" Game").to_string(),
        };
        let mut g = Game::new("gryphline", util::norm_title(&name).replace(' ', "-"), name);
        g.install_dir = Some(util::path_string(&path));
        g.launch = Launch::exe(util::path_string(&launcher));
        g.art.icon_file = Some(util::path_string(&exe));
        d.games.push(g);
    }
    Some(d)
}

fn root_exe(dir: &Path) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x.eq_ignore_ascii_case("exe")).unwrap_or(false))
        .max_by_key(|p| fs::metadata(p).map(|m| m.len()).unwrap_or(0))
}

/// Ankama Launcher (« zaap ») : `repositories\production\<jeu>\<branche>\release.json`.
fn ankama(ctx: &Ctx) -> Option<Detected> {
    let launcher = ctx
        .uninstall
        .iter()
        .find(|e| e.name.starts_with("Ankama Launcher"))
        .and_then(|e| e.icon.as_ref().and_then(|i| i.parent().map(|p| p.join("Ankama Launcher.exe"))))
        .filter(|p| p.is_file())?;
    let mut d = Detected::new("ankama", "Ankama").with_launcher_exe(&launcher);

    let repos = util::env_path("APPDATA")?.join("zaap").join("repositories").join("production");
    for game in fs::read_dir(&repos).into_iter().flatten().flatten() {
        for branch in fs::read_dir(game.path()).into_iter().flatten().flatten() {
            let Some(release) = fs::read_to_string(branch.path().join("release.json"))
                .ok()
                .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            else {
                continue;
            };
            let installed = release
                .get("installedFragments")
                .and_then(Value::as_array)
                .map(|a| !a.is_empty())
                .unwrap_or(false);
            if !installed {
                continue;
            }
            let game_name = release.get("gameName").and_then(Value::as_str).unwrap_or("").to_string();
            let branch_name = release.get("name").and_then(Value::as_str).unwrap_or("main");
            if game_name.is_empty() {
                continue;
            }
            let name = match branch_name {
                "beta" => format!("{game_name} (bêta)"),
                _ => game_name.clone(),
            };
            let key = release
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| format!("{game_name}-{branch_name}").to_lowercase());
            let mut g = Game::new("ankama", key, name);
            g.install_dir = release
                .get("location")
                .and_then(Value::as_str)
                .filter(|p| Path::new(p).is_dir())
                .map(str::to_string);
            // L'authentification passe obligatoirement par le launcher.
            g.launch = Launch::exe(util::path_string(&launcher));
            g.art.icon_file = Some(util::path_string(&launcher));
            d.games.push(g);
        }
    }
    Some(d)
}
