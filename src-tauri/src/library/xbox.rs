//! Microsoft Store / Xbox : les jeux sont suivis par Gaming Services dans le registre.
//!
//! `PackageRepository\Package` liste les paquets de jeux ; `GameConfig\<paquet>` donne le nom
//! affiché et les exécutables (absents pour les extensions/DLC, que l'on écarte ainsi).

use std::fs;
use std::path::{Path, PathBuf};

use winreg::RegKey;

use super::{Ctx, Detected};
use crate::model::{Game, Launch};
use crate::util;

const GAMING: &str = r"SOFTWARE\Microsoft\GamingServices";

pub fn scan(_ctx: &Ctx) -> Vec<Detected> {
    let Some(repo) = util::hklm_views(&format!(r"{GAMING}\PackageRepository")).into_iter().next() else {
        return Vec::new();
    };
    let mut d = Detected::new("xbox", "Xbox · Microsoft Store");
    d.installed = true;
    d.launcher = Launch::uri(r"shell:AppsFolder\Microsoft.GamingApp_8wekyb3d8bbwe!Microsoft.Xbox.App");
    d.icon_file = xbox_app_logo();

    let Ok(packages) = repo.open_subkey("Package") else { return vec![d] };
    let game_dirs = xbox_game_dirs();

    for (full_name, _) in packages.enum_values().flatten() {
        let Some(repo_id) = util::reg_string(&packages, &full_name) else { continue };
        // Jeux GDK : dossier déclaré par Gaming Services. Jeux UWP (Solitaire…) : dossier du paquet.
        let root = package_root(&repo, &repo_id, &full_name).or_else(|| {
            util::env_path("ProgramW6432")
                .or_else(|| util::env_path("ProgramFiles"))
                .map(|pf| pf.join("WindowsApps").join(&full_name))
                .filter(|p| p.is_dir())
        });
        let Some(root) = root else { continue };

        let config = util::hklm_views(&format!(r"{GAMING}\GameConfig\{full_name}")).into_iter().next();
        if let Some(cfg) = &config {
            // Extension de jeu : une config sans exécutable.
            let has_exe = cfg
                .open_subkey("Executable")
                .map(|k| k.enum_keys().flatten().next().is_some())
                .unwrap_or(false);
            if !has_exe {
                continue;
            }
        }

        let manifest = read_manifest(&root);
        let Some(app_id) = manifest.as_deref().and_then(application_id) else { continue };
        let Some(family) = family_name(&full_name) else { continue };

        let visuals = config.as_ref().and_then(|c| c.open_subkey("ShellVisuals").ok());
        let visual = |name: &str| visuals.as_ref().and_then(|k| util::reg_string(k, name));
        let name = visual("DefaultDisplayName")
            .or_else(|| manifest.as_deref().and_then(manifest_display_name))
            .unwrap_or_else(|| prettify_package(&full_name));

        let mut g = Game::new("xbox", &family, name.clone());
        g.launch = Launch::uri(format!(r"shell:AppsFolder\{family}!{app_id}"));
        // Dossier lisible par l'utilisateur quand le jeu est dans `X:\XboxGames`.
        let friendly = game_dirs.iter().map(|base| base.join(&name).join("Content")).find(|p| p.is_dir());
        g.install_dir = Some(util::path_string(friendly.as_deref().unwrap_or(&root)));

        let logo = ["Square480x480Logo", "Square150x150Logo"]
            .iter()
            .filter_map(|k| visual(k))
            .chain(
                manifest
                    .as_deref()
                    .and_then(|m| util::xml_attr(m, "Square150x150Logo"))
                    .map(str::to_string),
            )
            .find_map(|rel| resolve_logo(&root, &rel));
        g.art.icon_file = logo.map(|p| util::path_string(&p));
        d.games.push(g);
    }
    vec![d]
}

/// `PackageRepository\Root\<id>\<…>\Root` = dossier d'installation du paquet.
fn package_root(repo: &RegKey, repo_id: &str, full_name: &str) -> Option<PathBuf> {
    let roots = repo.open_subkey(format!(r"Root\{repo_id}")).ok()?;
    for sub in roots.enum_keys().flatten() {
        let Ok(k) = roots.open_subkey(&sub) else { continue };
        if util::reg_string(&k, "Package").as_deref() != Some(full_name) {
            continue;
        }
        if let Some(root) = util::reg_string(&k, "Root") {
            let path = PathBuf::from(root.trim_start_matches(r"\\?\").trim_end_matches('\\'));
            if path.is_dir() {
                return Some(path);
            }
        }
    }
    None
}

fn read_manifest(root: &Path) -> Option<String> {
    ["AppxManifest.xml", "appxmanifest.xml"]
        .iter()
        .find_map(|f| fs::read_to_string(root.join(f)).ok())
}

fn application_id(manifest: &str) -> Option<String> {
    let start = manifest.find("<Application ")?;
    let fragment = &manifest[start..];
    let end = fragment.find('>')?;
    util::xml_attr(&fragment[..end], "Id").map(str::to_string)
}

fn manifest_display_name(manifest: &str) -> Option<String> {
    let name = util::xml_tag(manifest, "DisplayName")?;
    // Les références `ms-resource:` demandent le moteur de ressources du paquet : on s'en passe.
    (!name.is_empty() && !name.starts_with("ms-resource")).then(|| util::xml_unescape(name))
}

/// `Nom_Version_Arch_Ressource_Editeur` → `Nom_Editeur`.
fn family_name(full_name: &str) -> Option<String> {
    let parts: Vec<&str> = full_name.split('_').collect();
    (parts.len() >= 5).then(|| format!("{}_{}", parts[0], parts[parts.len() - 1]))
}

/// `Microsoft.MicrosoftSolitaireCollection_…` → « Microsoft Solitaire Collection ».
fn prettify_package(full_name: &str) -> String {
    let raw = full_name.split('_').next().unwrap_or(full_name);
    let raw = raw.rsplit('.').next().unwrap_or(raw);
    let mut out = String::new();
    let mut prev_lower = false;
    for c in raw.chars() {
        if c.is_uppercase() && prev_lower {
            out.push(' ');
        }
        prev_lower = c.is_lowercase();
        out.push(c);
    }
    out
}

/// Les logos existent souvent seulement en variantes `.scale-200.png` ou `.targetsize-48.png`.
fn resolve_logo(root: &Path, relative: &str) -> Option<PathBuf> {
    let direct = root.join(relative.replace('/', "\\"));
    if direct.is_file() {
        return Some(direct);
    }
    let stem = direct.file_stem()?.to_string_lossy().to_string();
    let ext = direct.extension()?.to_string_lossy().to_string();
    let parent = direct.parent()?;
    [
        "scale-200",
        "scale-400",
        "scale-150",
        "scale-125",
        "scale-100",
        "targetsize-96",
        "targetsize-64",
        "targetsize-48",
        "targetsize-256",
    ]
    .iter()
    .map(|variant| parent.join(format!("{stem}.{variant}.{ext}")))
    .find(|p| p.is_file())
}

/// Logo de l'application Xbox, pris dans son paquet installé (pictogramme de la source).
fn xbox_app_logo() -> Option<PathBuf> {
    let packages = util::hkcu(
        r"Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppModel\Repository\Packages",
    )?;
    let name = packages
        .enum_keys()
        .flatten()
        .find(|k| k.starts_with("Microsoft.GamingApp_") && k.contains("_x64_"))?;
    let root = PathBuf::from(util::reg_string(&packages.open_subkey(name).ok()?, "PackageRootFolder")?);
    let manifest = read_manifest(&root)?;
    ["Square44x44Logo", "Square150x150Logo"]
        .iter()
        .filter_map(|attr| util::xml_attr(&manifest, attr))
        .find_map(|rel| resolve_logo(&root, rel))
}

/// Dossiers `XboxGames` des lecteurs marqués par un fichier `.GamingRoot`.
fn xbox_game_dirs() -> Vec<PathBuf> {
    (b'A'..=b'Z')
        .map(|letter| format!("{}:\\", letter as char))
        .filter(|drive| Path::new(drive).join(".GamingRoot").is_file())
        .map(|drive| PathBuf::from(drive).join("XboxGames"))
        .filter(|p| p.is_dir())
        .collect()
}
