//! Steam : `libraryfolders.vdf` → `appmanifest_*.acf`, visuels depuis le cache local du client.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::{Ctx, Detected};
use crate::model::{Game, Launch};
use crate::util::{self, Vdf};

/// Outils sans intérêt dans une bibliothèque de jeux (redistribuables, runtimes Proton…).
fn is_runtime(appid: u32, name: &str) -> bool {
    appid == 228980
        || name.starts_with("Steamworks Common")
        || name.starts_with("Proton ")
        || name.starts_with("Steam Linux Runtime")
}

/// Logiciels vendus sur Steam mais qui ne sont pas des jeux : masqués par défaut.
const TOOLS: &[u32] = &[250820, 431960, 365670, 993090, 629520];

fn steam_root() -> Option<PathBuf> {
    let from_user = util::hkcu(r"Software\Valve\Steam").and_then(|k| util::reg_string(&k, "SteamPath"));
    let raw = from_user.or_else(|| util::hklm_string(r"SOFTWARE\Valve\Steam", "InstallPath"))?;
    let path = PathBuf::from(raw.replace('/', "\\"));
    path.is_dir().then_some(path)
}

pub fn scan(_ctx: &Ctx) -> Vec<Detected> {
    let Some(root) = steam_root() else { return Vec::new() };
    let mut d = Detected::new("steam", "Steam").with_launcher_exe(&root.join("steam.exe"));

    // Bibliothèques (le dossier Steam lui-même + celles déclarées).
    let mut libs = vec![root.clone()];
    if let Some(vdf) = fs::read_to_string(root.join("steamapps").join("libraryfolders.vdf"))
        .ok()
        .and_then(|t| util::parse_vdf(&t))
    {
        if let Some(folders) = vdf.get("libraryfolders") {
            for (_, entry) in folders.entries() {
                if let Some(p) = entry.str_of("path") {
                    let p = PathBuf::from(p);
                    if !libs.iter().any(|l| util::path_key(l) == util::path_key(&p)) {
                        libs.push(p);
                    }
                }
            }
        }
    }

    let stats = local_stats(&root);
    let art_cache = root.join("appcache").join("librarycache");

    for lib in &libs {
        let apps = lib.join("steamapps");
        let Ok(entries) = fs::read_dir(&apps) else { continue };
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            if !(file_name.starts_with("appmanifest_") && file_name.ends_with(".acf")) {
                continue;
            }
            let Some(vdf) = fs::read_to_string(entry.path()).ok().and_then(|t| util::parse_vdf(&t)) else {
                continue;
            };
            let Some(app) = vdf.get("AppState") else { continue };
            let Some(appid) = app.str_of("appid").and_then(|s| s.parse::<u32>().ok()) else { continue };
            let name = app.str_of("name").unwrap_or("").to_string();
            if name.is_empty() || is_runtime(appid, &name) {
                continue;
            }

            let mut g = Game::new("steam", appid.to_string(), name);
            g.is_tool = TOOLS.contains(&appid);
            g.launch = Launch::uri(format!("steam://rungameid/{appid}"));
            if let Some(dir) = app.str_of("installdir") {
                g.install_dir = Some(util::path_string(&apps.join("common").join(dir)));
            }
            g.size_bytes = app.str_of("SizeOnDisk").and_then(|s| s.parse().ok()).filter(|n| *n > 0);
            let manifest_played = app.str_of("LastPlayed").and_then(|s| s.parse::<u64>().ok());
            let (playtime, config_played) = stats.get(&appid).copied().unwrap_or((None, None));
            g.last_played = manifest_played.into_iter().chain(config_played).max().filter(|t| *t > 0);
            g.playtime_min = playtime.filter(|m| *m > 0);

            g.art.steam_appid = Some(appid);
            let art = local_art(&art_cache.join(appid.to_string()));
            g.art.cover_file = art.cover;
            g.art.hero_file = art.hero;
            g.art.logo_file = art.logo;
            d.games.push(g);
        }
    }
    vec![d]
}

#[derive(Default)]
struct LocalArt {
    cover: Option<String>,
    hero: Option<String>,
    logo: Option<String>,
}

/// Le client range ses visuels soit à la racine du dossier de l'app, soit dans des sous-dossiers
/// nommés par empreinte (jeux récents) : on regarde les deux niveaux.
fn local_art(dir: &Path) -> LocalArt {
    let mut art = LocalArt::default();
    let mut capsule: Option<String> = None;
    let mut visit = |path: &Path| {
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { return };
        let value = || Some(util::path_string(path));
        match name {
            "library_600x900.jpg" => art.cover = value(),
            "library_capsule.jpg" => capsule = value(),
            "library_hero.jpg" => art.hero = value(),
            "logo.png" => art.logo = value(),
            _ => {}
        }
    };
    let Ok(entries) = fs::read_dir(dir) else { return art };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Ok(sub) = fs::read_dir(&path) {
                for e in sub.flatten() {
                    visit(&e.path());
                }
            }
        } else {
            visit(&path);
        }
    }
    if art.cover.is_none() {
        art.cover = capsule;
    }
    art
}

/// Temps de jeu (minutes) et dernière session, lus dans le `localconfig.vdf` du compte le plus récent.
fn local_stats(root: &Path) -> HashMap<u32, (Option<u64>, Option<u64>)> {
    let mut out = HashMap::new();
    let Ok(users) = fs::read_dir(root.join("userdata")) else { return out };
    let newest = users
        .flatten()
        .map(|u| u.path().join("config").join("localconfig.vdf"))
        .filter(|p| p.is_file())
        .max_by_key(|p| util::mtime_secs(p));
    let Some(vdf) = newest
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|t| util::parse_vdf(&t))
    else {
        return out;
    };
    let apps = vdf.path(&["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"]);
    for (id, node) in apps.map(Vdf::entries).unwrap_or(&[]) {
        let Ok(appid) = id.parse::<u32>() else { continue };
        let num = |k: &str| node.str_of(k).and_then(|s| s.parse::<u64>().ok());
        out.insert(appid, (num("Playtime"), num("LastPlayed")));
    }
    out
}
