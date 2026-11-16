//! Vue « Stockage » : espace de chaque disque et place occupée par chaque jeu.
//!
//! Steam, Epic et EA donnent la taille de leurs jeux ; pour les autres, on additionne la taille des
//! fichiers du dossier d'installation (une fois par session, en parallèle, puis mis en cache).

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{LazyLock, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Manager};
use windows::core::HSTRING;
use windows::Win32::Storage::FileSystem::{GetDiskFreeSpaceExW, GetVolumeInformationW};

use crate::state::AppState;

static MEASURED: LazyLock<Mutex<HashMap<String, u64>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Disk {
    root: String,
    label: String,
    total: u64,
    free: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageGame {
    id: String,
    root: String,
    size: u64,
    /// Taille calculée par Launcher Pro (le launcher ne la fournit pas).
    measured: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageInfo {
    disks: Vec<Disk>,
    games: Vec<StorageGame>,
}

/// `D:\Jeux\…` → `D:\`.
fn drive_root(dir: &str) -> Option<String> {
    let bytes = dir.as_bytes();
    (bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
        .then(|| format!("{}:\\", (bytes[0] as char).to_ascii_uppercase()))
}

fn disk(root: &str) -> Option<Disk> {
    let path = HSTRING::from(root);
    let (mut free, mut total) = (0u64, 0u64);
    let mut name = [0u16; 261];
    unsafe {
        GetDiskFreeSpaceExW(&path, Some(&mut free as *mut u64), Some(&mut total as *mut u64), None).ok()?;
        let _ = GetVolumeInformationW(&path, Some(&mut name), None, None, None, None);
    }
    let len = name.iter().position(|c| *c == 0).unwrap_or(0);
    Some(Disk {
        root: root.to_string(),
        label: String::from_utf16_lossy(&name[..len]),
        total,
        free,
    })
}

/// Somme des fichiers d'un dossier (sans suivre les liens et jonctions, qui pourraient boucler).
fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                stack.push(entry.path());
            } else if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

#[tauri::command]
pub async fn storage_info(app: AppHandle) -> Result<StorageInfo, String> {
    // (id, dossier, taille donnée par le launcher)
    let games: Vec<(String, String, Option<u64>)> = {
        let state = app.state::<AppState>();
        let unlocked = state.hidden_unlocked.load(Ordering::Relaxed);
        let library = state.library_read();
        let prefs = state.prefs_read();
        library
            .games
            .iter()
            .filter(|g| unlocked || !prefs.hidden.contains(&g.id))
            .filter_map(|g| g.install_dir.clone().map(|dir| (g.id.clone(), dir, g.size_bytes)))
            .collect()
    };

    tauri::async_runtime::spawn_blocking(move || {
        // Mesure en parallèle des dossiers dont on ne connaît pas encore la taille.
        let to_measure: Vec<(String, PathBuf)> = {
            let cache = MEASURED.lock().unwrap_or_else(|e| e.into_inner());
            games
                .iter()
                .filter(|(id, dir, size)| size.is_none() && !cache.contains_key(id) && Path::new(dir).is_dir())
                .map(|(id, dir, _)| (id.clone(), PathBuf::from(dir)))
                .collect()
        };
        let results: Vec<(String, u64)> = std::thread::scope(|scope| {
            let handles: Vec<_> = to_measure
                .iter()
                .map(|(id, dir)| scope.spawn(move || (id.clone(), dir_size(dir))))
                .collect();
            handles.into_iter().filter_map(|h| h.join().ok()).collect()
        });
        let cache = {
            let mut cache = MEASURED.lock().unwrap_or_else(|e| e.into_inner());
            cache.extend(results);
            cache.clone()
        };

        let mut roots = BTreeMap::new();
        let mut list = Vec::new();
        for (id, dir, size) in games {
            let Some(root) = drive_root(&dir) else { continue };
            let (size, measured) = match size {
                Some(s) => (s, false),
                None => (cache.get(&id).copied().unwrap_or(0), true),
            };
            roots.insert(root.clone(), ());
            list.push(StorageGame { id, root, size, measured });
        }
        list.sort_by(|a, b| b.size.cmp(&a.size));
        StorageInfo {
            disks: roots.keys().filter_map(|r| disk(r)).collect(),
            games: list,
        }
    })
    .await
    .map_err(|e| e.to_string())
}

/// Oublie les tailles mesurées (après un rescan, un jeu a pu grossir).
pub fn forget_measures() {
    MEASURED.lock().unwrap_or_else(|e| e.into_inner()).clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util;

    #[test]
    fn roots() {
        assert_eq!(drive_root(r"d:\SteamLibrary\x").as_deref(), Some("D:\\"));
        assert_eq!(drive_root(r"\\serveur\jeux"), None);
    }

    #[test]
    fn measures_a_folder() {
        let dir = std::env::temp_dir().join(format!("lp-size-{}", util::now_secs()));
        std::fs::create_dir_all(dir.join("sous")).unwrap();
        std::fs::write(dir.join("a.bin"), vec![0u8; 1000]).unwrap();
        std::fs::write(dir.join("sous").join("b.bin"), vec![0u8; 500]).unwrap();
        assert_eq!(dir_size(&dir), 1500);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(disk("C:\\").map(|d| d.total > 0).unwrap_or(false));
    }
}
