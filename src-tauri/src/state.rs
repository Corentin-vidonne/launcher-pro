//! État partagé de l'application et persistance (préférences, bibliothèque en cache).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Instant;

use serde::de::DeserializeOwned;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::art::{self, ArtStore};
use crate::model::{Game, GameDto, Library, LibraryDto, Prefs, SourceDto};
use crate::store::{Region, USER_AGENT};
use crate::{library, util};

pub struct AppState {
    data_dir: PathBuf,
    pub library: RwLock<Library>,
    pub prefs: RwLock<Prefs>,
    pub art: ArtStore,
    pub http: reqwest::Client,
    pub region: Region,
    /// Réponses du magasin gardées quelques minutes (clé → instant + JSON).
    pub store_cache: Mutex<HashMap<String, (Instant, serde_json::Value)>>,
    scanning: AtomicBool,
    /// Coffre des jeux masqués ouvert (mot de passe saisi) ; toujours fermé au démarrage.
    pub hidden_unlocked: AtomicBool,
}

fn read_json<T: DeserializeOwned + Default>(path: PathBuf) -> T {
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn write_json<T: Serialize>(path: PathBuf, value: &T) {
    if let Ok(json) = serde_json::to_vec(value) {
        // Écriture atomique : un plantage ne laisse jamais un fichier tronqué.
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(tmp, path);
        }
    }
}

impl AppState {
    pub fn init(data_dir: PathBuf, cache_dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&data_dir);
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .gzip(true)
            .connect_timeout(std::time::Duration::from_secs(6))
            .timeout(std::time::Duration::from_secs(15))
            .pool_idle_timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        AppState {
            library: RwLock::new(read_json(data_dir.join("library.json"))),
            prefs: RwLock::new(read_json(data_dir.join("prefs.json"))),
            art: ArtStore::load(cache_dir.join("art")),
            http,
            region: Region::detect(),
            store_cache: Mutex::new(HashMap::new()),
            // Vrai au départ : un premier scan est lancé dès l'ouverture.
            scanning: AtomicBool::new(true),
            hidden_unlocked: AtomicBool::new(false),
            data_dir,
        }
    }

    pub fn prefs_read(&self) -> RwLockReadGuard<'_, Prefs> {
        self.prefs.read().unwrap_or_else(|e| e.into_inner())
    }

    pub fn prefs_write(&self) -> RwLockWriteGuard<'_, Prefs> {
        self.prefs.write().unwrap_or_else(|e| e.into_inner())
    }

    pub fn library_read(&self) -> RwLockReadGuard<'_, Library> {
        self.library.read().unwrap_or_else(|e| e.into_inner())
    }

    pub fn save_prefs(&self) {
        write_json(self.data_dir.join("prefs.json"), &*self.prefs_read());
    }

    fn save_library(&self) {
        write_json(self.data_dir.join("library.json"), &*self.library_read());
    }

    pub fn find_game(&self, id: &str) -> Option<Game> {
        self.library_read().games.iter().find(|g| g.id == id).cloned()
    }

    pub fn game_dto(&self, game: &Game) -> GameDto {
        let prefs = self.prefs_read();
        self.game_dto_with(game, &prefs)
    }

    fn game_dto_with(&self, game: &Game, prefs: &Prefs) -> GameDto {
        let r = art::resolve(&self.art, game);
        let link = |kind: &str, path: &Option<PathBuf>| path.as_ref().map(|p| art::url(kind, &game.id, p));
        GameDto {
            id: game.id.clone(),
            source: game.source.clone(),
            name: game.name.clone(),
            install_dir: game.install_dir.clone(),
            size_bytes: game.size_bytes,
            // La session la plus récente gagne, qu'elle vienne du launcher d'origine ou d'ici.
            last_played: game
                .last_played
                .into_iter()
                .chain(prefs.launched.get(&game.id).copied())
                .max(),
            playtime_min: game.playtime_min,
            is_tool: game.is_tool,
            favorite: prefs.favorites.contains(&game.id),
            hidden: prefs.hidden.contains(&game.id),
            can_launch: game.launch.is_some(),
            cover: link("cover", &r.cover),
            hero: link("hero", &r.hero),
            logo: link("logo", &r.logo),
            icon: link("icon", &r.icon),
            custom_art: {
                let entry = self.art.entry(&game.id);
                entry.custom_cover.is_some() || entry.custom_hero.is_some()
            },
        }
    }

    pub fn library_dto(&self) -> LibraryDto {
        let library = self.library_read();
        let prefs = self.prefs_read();
        // Les jeux masqués ne sont envoyés à l'interface que coffre déverrouillé (onglet « Masqués »).
        let unlocked = self.hidden_unlocked.load(Ordering::Relaxed);
        let shown = |g: &&Game| !prefs.hidden.contains(&g.id);
        let games: Vec<GameDto> = library
            .games
            .iter()
            .filter(|g| unlocked || shown(g))
            .map(|g| self.game_dto_with(g, &prefs))
            .collect();
        let sources = library
            .sources
            .iter()
            .map(|s| SourceDto {
                id: s.id.clone(),
                name: s.name.clone(),
                installed: s.installed,
                can_open: s.launcher.is_some(),
                count: library.games.iter().filter(shown).filter(|g| g.source == s.id).count(),
                icon: art::source_icon(&self.art, s).map(|p| art::url("source", &s.id, &p)),
            })
            .collect();
        LibraryDto {
            games,
            sources,
            scanned_at: library.scanned_at,
            scanning: self.scanning.load(Ordering::Relaxed),
        }
    }
}

/// Rescanne le PC, publie la bibliothèque puis complète les visuels en tâche de fond.
pub async fn refresh(app: AppHandle) -> LibraryDto {
    let custom = app.state::<AppState>().prefs_read().custom.clone();
    app.state::<AppState>().scanning.store(true, Ordering::Relaxed);
    let scanned = tauri::async_runtime::spawn_blocking(move || library::scan_all(&custom)).await;
    let state = app.state::<AppState>();
    if let Ok(scanned) = scanned {
        *state.library.write().unwrap_or_else(|e| e.into_inner()) = scanned;
        state.save_library();
    }
    state.scanning.store(false, Ordering::Relaxed);
    crate::storage::forget_measures();
    let dto = state.library_dto();
    let _ = app.emit("library-updated", &dto);
    tauri::async_runtime::spawn(art::fill_missing(app.clone()));
    dto
}

pub fn mark_launched(state: &AppState, id: &str) {
    state.prefs_write().launched.insert(id.to_string(), util::now_secs());
    state.save_prefs();
}
