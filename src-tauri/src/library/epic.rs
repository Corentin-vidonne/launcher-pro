//! Epic Games : manifestes `.item` + cache catalogue du launcher pour les visuels.

use std::fs;
use std::path::PathBuf;

use base64::Engine;
use serde_json::Value;

use super::{Ctx, Detected};
use crate::model::{Game, Launch};
use crate::util;

fn launcher_exe() -> Option<PathBuf> {
    let from_eos = util::hkcu(r"Software\Epic Games\EOS")
        .and_then(|k| util::reg_string(&k, "ModSdkCommand"))
        .map(|s| PathBuf::from(s.replace('/', "\\")));
    let from_install = util::hklm_string(r"SOFTWARE\EpicGames\Unreal Engine", "INSTALLDIR").map(|dir| {
        PathBuf::from(dir)
            .join("Launcher")
            .join("Portal")
            .join("Binaries")
            .join("Win64")
            .join("EpicGamesLauncher.exe")
    });
    from_eos.into_iter().chain(from_install).find(|p| p.is_file())
}

fn data_dir() -> PathBuf {
    util::hklm_string(r"SOFTWARE\Epic Games\EpicGamesLauncher", "AppDataPath")
        .map(PathBuf::from)
        .unwrap_or_else(|| util::program_data().join("Epic").join("EpicGamesLauncher").join("Data"))
}

pub fn scan(_ctx: &Ctx) -> Vec<Detected> {
    let data = data_dir();
    let manifests = data.join("Manifests");
    let exe = launcher_exe();
    if exe.is_none() && !manifests.is_dir() {
        return Vec::new();
    }
    let mut d = Detected::new("epic", "Epic Games");
    d.installed = true;
    if let Some(exe) = &exe {
        d = d.with_launcher_exe(exe);
    } else {
        d.launcher = Launch::uri("com.epicgames.launcher://store");
    }

    let mut catalog: Option<Vec<Value>> = None; // chargé seulement s'il y a des jeux
    for entry in fs::read_dir(&manifests).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("item") {
            continue;
        }
        let Some(m) = fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        else {
            continue;
        };
        let text = |k: &str| m.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        let app_name = text("AppName");
        let display = text("DisplayName");
        let exe_rel = text("LaunchExecutable");
        // Contenus additionnels : pas d'exécutable, ou rattachés à un autre jeu.
        let main_app = text("MainGameAppName");
        if app_name.is_empty()
            || display.is_empty()
            || exe_rel.is_empty()
            || (!main_app.is_empty() && main_app != app_name)
        {
            continue;
        }
        let categories: Vec<&str> = m
            .get("AppCategories")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        if !categories.is_empty() && !categories.contains(&"games") {
            continue;
        }

        let namespace = text("CatalogNamespace");
        let item_id = text("CatalogItemId");
        let mut g = Game::new("epic", &app_name, display);
        g.install_dir = Some(text("InstallLocation")).filter(|s| !s.is_empty());
        g.size_bytes = m.get("InstallSize").and_then(Value::as_u64).filter(|n| *n > 0);
        g.launch = Launch::uri(format!(
            "com.epicgames.launcher://apps/{namespace}%3A{item_id}%3A{app_name}?action=launch&silent=true"
        ));
        if let Some(dir) = &g.install_dir {
            g.art.icon_file = Some(util::path_string(&PathBuf::from(dir).join(exe_rel.replace('/', "\\"))));
        }

        let cat = catalog.get_or_insert_with(|| load_catalog(&data));
        if let Some(images) = cat
            .iter()
            .find(|e| e.get("id").and_then(Value::as_str) == Some(item_id.as_str()))
            .and_then(|e| e.get("keyImages"))
            .and_then(Value::as_array)
        {
            let url_of = |kind: &str| {
                images
                    .iter()
                    .find(|i| i.get("type").and_then(Value::as_str) == Some(kind))
                    .and_then(|i| i.get("url"))
                    .and_then(Value::as_str)
            };
            // Le CDN Epic redimensionne à la volée : inutile de télécharger du 1200×1600.
            g.art.cover_url = url_of("DieselGameBoxTall").map(|u| format!("{u}?h=900&resize=1&w=600"));
            g.art.hero_url = url_of("DieselGameBox").map(|u| format!("{u}?h=1080&resize=1&w=1920"));
        }
        d.games.push(g);
    }
    vec![d]
}

/// `catcache.bin` = JSON encodé en base64 décrivant les jeux possédés (dont leurs visuels).
fn load_catalog(data: &std::path::Path) -> Vec<Value> {
    fs::read(data.join("Catalog").join("catcache.bin"))
        .ok()
        .and_then(|raw| {
            let trimmed: Vec<u8> = raw.into_iter().filter(|b| !b.is_ascii_whitespace()).collect();
            base64::engine::general_purpose::STANDARD.decode(trimmed).ok()
        })
        .and_then(|bytes| serde_json::from_slice::<Vec<Value>>(&bytes).ok())
        .unwrap_or_default()
}
