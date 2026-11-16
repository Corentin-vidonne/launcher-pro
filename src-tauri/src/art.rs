//! Visuels des jeux : résolution locale, téléchargement différé, cache disque, service à l'interface.
//!
//! L'interface charge les images via `http://art.localhost/<type>/<id>` : le fichier est lu depuis
//! le disque à la demande et mis en cache par le moteur web (aucune image ne transite par l'IPC).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::http::{Response, StatusCode};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Semaphore;

use crate::model::{Game, Source};
use crate::state::AppState;
use crate::store::{self, Region};
use crate::{shell, util};

/// Délai avant de retenter une recherche de jaquette restée sans résultat.
const RETRY_SECS: u64 = 7 * 24 * 3600;
const MAX_IMAGE_BYTES: usize = 12 * 1024 * 1024;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ArtEntry {
    pub cover: Option<String>,
    pub hero: Option<String>,
    pub icon: Option<String>,
    /// Dernière tentative de recherche en ligne (secondes Unix).
    pub tried: u64,
    /// Images choisies par l'utilisateur : prioritaires sur tout le reste.
    pub custom_cover: Option<String>,
    pub custom_hero: Option<String>,
}

pub struct ArtStore {
    dir: PathBuf,
    index: RwLock<HashMap<String, ArtEntry>>,
}

impl ArtStore {
    pub fn load(dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&dir);
        let index = std::fs::read(dir.join("index.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        ArtStore { dir, index: RwLock::new(index) }
    }

    pub fn save(&self) {
        let index = self.index.read().unwrap_or_else(|e| e.into_inner());
        if let Ok(json) = serde_json::to_vec(&*index) {
            let _ = std::fs::write(self.dir.join("index.json"), json);
        }
    }

    pub fn entry(&self, id: &str) -> ArtEntry {
        self.index
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(id)
            .cloned()
            .unwrap_or_default()
    }

    fn set(&self, id: &str, entry: ArtEntry) {
        self.index
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id.to_string(), entry);
    }

    fn file_for(&self, id: &str, kind: &str, ext: &str) -> PathBuf {
        let safe: String = id
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        self.dir.join(format!("{safe}.{kind}.{ext}"))
    }
}

#[derive(Default)]
pub struct Resolved {
    pub cover: Option<PathBuf>,
    pub hero: Option<PathBuf>,
    pub logo: Option<PathBuf>,
    pub icon: Option<PathBuf>,
}

fn existing(path: &Option<String>) -> Option<PathBuf> {
    path.as_ref().map(PathBuf::from).filter(|p| p.is_file())
}

fn is_image(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).map(str::to_lowercase).as_deref(),
        Some("ico" | "png" | "jpg" | "jpeg" | "webp" | "bmp")
    )
}

/// Fichiers disponibles tout de suite pour un jeu (local d'abord, cache ensuite).
pub fn resolve(store: &ArtStore, game: &Game) -> Resolved {
    let entry = store.entry(&game.id);
    Resolved {
        cover: existing(&entry.custom_cover)
            .or_else(|| existing(&game.art.cover_file))
            .or_else(|| existing(&entry.cover)),
        hero: existing(&entry.custom_hero)
            .or_else(|| existing(&game.art.hero_file))
            .or_else(|| existing(&entry.hero)),
        logo: existing(&game.art.logo_file),
        icon: existing(&game.art.icon_file)
            .filter(|p| is_image(p))
            .or_else(|| existing(&entry.icon)),
    }
}

pub fn source_icon(store: &ArtStore, source: &Source) -> Option<PathBuf> {
    existing(&source.icon_file)
        .filter(|p| is_image(p))
        .or_else(|| existing(&store.entry(&format!("source:{}", source.id)).icon))
}

/// URL servie par le protocole `art`. Le paramètre `v` invalide le cache quand le fichier change.
pub fn url(kind: &str, id: &str, path: &Path) -> String {
    format!(
        "http://art.localhost/{kind}/{}?v={}",
        utf8_percent_encode(id, NON_ALPHANUMERIC),
        util::mtime_secs(path)
    )
}

// ───────────────────────────── Protocole `art://` ─────────────────────────────

pub async fn serve(app: &AppHandle, path: &str) -> Response<Vec<u8>> {
    let not_found = || {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Vec::new())
            .unwrap_or_default()
    };
    let mut parts = path.trim_start_matches('/').splitn(2, '/');
    let (Some(kind), Some(raw_id)) = (parts.next(), parts.next()) else { return not_found() };
    let id = percent_decode_str(raw_id).decode_utf8_lossy().to_string();

    // Seuls les fichiers rattachés à un jeu ou une source connus sont servis.
    let file = {
        let state = app.state::<AppState>();
        let library = state.library.read().unwrap_or_else(|e| e.into_inner());
        if kind == "source" {
            library.sources.iter().find(|s| s.id == id).and_then(|s| source_icon(&state.art, s))
        } else {
            library.games.iter().find(|g| g.id == id).and_then(|g| {
                let r = resolve(&state.art, g);
                match kind {
                    "cover" => r.cover,
                    "hero" => r.hero,
                    "logo" => r.logo,
                    "icon" => r.icon,
                    _ => None,
                }
            })
        }
    };
    let Some(file) = file else { return not_found() };
    let Ok(bytes) = tokio::fs::read(&file).await else { return not_found() };
    let mime = match file.extension().and_then(|e| e.to_str()).map(str::to_lowercase).as_deref() {
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("webp") => "image/webp",
        Some("avif") => "image/avif",
        Some("bmp") => "image/bmp",
        _ => "image/jpeg",
    };
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", mime)
        .header("Cache-Control", "public, max-age=31536000, immutable")
        .header("Access-Control-Allow-Origin", "*")
        .body(bytes)
        .unwrap_or_default()
}

// ───────────────────────────── Complétion en arrière-plan ─────────────────────────────

/// Extrait les icônes manquantes et télécharge les jaquettes absentes, puis prévient l'interface.
/// Ne fait rien (et aucun appel réseau) quand tout est déjà en cache.
pub async fn fill_missing(app: AppHandle) {
    let (games, sources) = {
        let state = app.state::<AppState>();
        let library = state.library.read().unwrap_or_else(|e| e.into_inner());
        (library.games.clone(), library.sources.clone())
    };
    let now = util::now_secs();

    // Icônes des launchers (barre latérale).
    let mut sources_changed = false;
    for source in &sources {
        let state = app.state::<AppState>();
        if source_icon(&state.art, source).is_some() {
            continue;
        }
        let Some(exe) = existing(&source.icon_file) else { continue };
        let key = format!("source:{}", source.id);
        let target = state.art.file_for(&key, "icon", "png");
        if extract_icon(exe, target.clone(), 64).await {
            let mut entry = state.art.entry(&key);
            entry.icon = Some(util::path_string(&target));
            state.art.set(&key, entry);
            sources_changed = true;
        }
    }
    if sources_changed {
        let state = app.state::<AppState>();
        let _ = app.emit("library-updated", state.library_dto());
    }

    // Jeux : au plus 4 résolutions simultanées pour rester discret sur le réseau.
    let limiter = Arc::new(Semaphore::new(4));
    let mut tasks = tokio::task::JoinSet::new();
    for game in games {
        let (need_icon, need_remote) = {
            let state = app.state::<AppState>();
            let entry = state.art.entry(&game.id);
            let r = resolve(&state.art, &game);
            let icon_source = existing(&game.art.icon_file).filter(|p| !is_image(p));
            (
                // Sans jaquette, on se rabat sur l'icône de l'exécutable.
                r.icon.is_none() && icon_source.is_some(),
                (r.cover.is_none() || r.hero.is_none()) && now.saturating_sub(entry.tried) > RETRY_SECS,
            )
        };
        if !need_icon && !need_remote {
            continue;
        }
        let app = app.clone();
        let limiter = limiter.clone();
        tasks.spawn(async move {
            let _permit = limiter.acquire().await;
            let state = app.state::<AppState>();
            let mut entry = state.art.entry(&game.id);

            if need_icon {
                if let Some(exe) = existing(&game.art.icon_file) {
                    let target = state.art.file_for(&game.id, "icon", "png");
                    if extract_icon(exe, target.clone(), 256).await {
                        entry.icon = Some(util::path_string(&target));
                    }
                }
            }
            if need_remote {
                let current = resolve(&state.art, &game);
                let (cover_url, hero_url) = find_remote(&state.http, &state.region, &game).await;
                if current.cover.is_none() {
                    if let Some(url) = cover_url {
                        entry.cover = download(&state.http, &url, &state.art, &game.id, "cover").await;
                    }
                }
                if current.hero.is_none() {
                    if let Some(url) = hero_url {
                        entry.hero = download(&state.http, &url, &state.art, &game.id, "hero").await;
                    }
                }
                entry.tried = now;
            }
            state.art.set(&game.id, entry);
            let _ = app.emit("art-updated", state.game_dto(&game));
        });
    }
    let mut changed = false;
    while tasks.join_next().await.is_some() {
        changed = true;
    }
    if changed || sources_changed {
        app.state::<AppState>().art.save();
    }
}

async fn extract_icon(source: PathBuf, target: PathBuf, size: i32) -> bool {
    tauri::async_runtime::spawn_blocking(move || {
        shell::icon_png(&source, size)
            .map(|png| std::fs::write(&target, png).is_ok())
            .unwrap_or(false)
    })
    .await
    .unwrap_or(false)
}

/// Cherche jaquette (portrait) et bannière (paysage) pour un jeu sans visuel local.
async fn find_remote(http: &Client, region: &Region, game: &Game) -> (Option<String>, Option<String>) {
    let mut cover = game.art.cover_url.clone();
    let mut hero = game.art.hero_url.clone();
    if cover.is_some() && hero.is_some() {
        return (cover, hero);
    }
    if let Some(appid) = game.art.steam_appid {
        if let Some(a) = store::steam::assets(http, region, &[appid]).await.remove(&appid) {
            cover = cover.or(a.cover);
            hero = hero.or(a.hero);
        }
        return (cover, hero);
    }
    // Rapprochement par titre exact (normalisé) : mieux vaut une icône qu'une mauvaise jaquette.
    let key = util::match_key(&game.name);
    let order: &[&str] = if game.source == "xbox" {
        &["microsoft", "steam"]
    } else {
        &["steam", "microsoft"]
    };
    // Deux formulations : le nom tel quel, puis sans suffixe de région/édition
    // (« World of Tanks EU » ne renvoie rien, « world of tanks » si).
    let plain: String = game.name.chars().filter(|c| !matches!(c, '™' | '®' | '©')).collect();
    let mut queries = vec![plain];
    if util::norm_title(&queries[0]) != key {
        queries.push(key.clone());
    }
    for store_id in order {
        for query in &queries {
            if cover.is_some() && hero.is_some() {
                break;
            }
            let items = match *store_id {
                "steam" => store::steam::search(http, region, query).await,
                _ => store::microsoft::search(http, region, query).await,
            }
            .unwrap_or_default();
            if let Some(item) = items.into_iter().find(|i| util::match_key(&i.title) == key) {
                cover = cover.or(item.cover);
                hero = hero.or(item.hero);
                break;
            }
        }
    }
    (cover, hero)
}

async fn download(http: &Client, url: &str, store: &ArtStore, id: &str, kind: &str) -> Option<String> {
    let resp = http.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let bytes = resp.bytes().await.ok()?;
    if bytes.len() < 128 || bytes.len() > MAX_IMAGE_BYTES {
        return None;
    }
    // On se fie au contenu, pas à l'en-tête : seules de vraies images sont écrites sur disque.
    let ext = image_ext(&bytes)?;
    let target = store.file_for(id, kind, ext);
    tokio::fs::write(&target, &bytes).await.ok()?;
    Some(util::path_string(&target))
}

/// Format d'image d'après son contenu (jamais d'après l'extension ou l'en-tête HTTP).
fn image_ext(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() < 12 {
        return None;
    }
    Some(match &bytes[..12] {
        [0xFF, 0xD8, 0xFF, ..] => "jpg",
        [0x89, b'P', b'N', b'G', ..] => "png",
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P'] => "webp",
        [_, _, _, _, b'f', b't', b'y', b'p', b'a', b'v', b'i', b'f'] => "avif",
        _ => return None,
    })
}

/// Jaquette (`cover`) ou fond (`hero`) choisi par l'utilisateur ; `None` = retour à l'automatique.
/// L'image est copiée dans le cache : le fichier d'origine peut ensuite être déplacé ou supprimé.
pub fn set_custom(store: &ArtStore, id: &str, kind: &str, source: Option<&Path>) -> Result<(), String> {
    let mut entry = store.entry(id);
    let slot = match kind {
        "cover" => &mut entry.custom_cover,
        "hero" => &mut entry.custom_hero,
        _ => return Err("Type d'image inconnu.".into()),
    };
    if let Some(old) = slot.take() {
        let _ = std::fs::remove_file(old);
    }
    if let Some(source) = source {
        let bytes = std::fs::read(source).map_err(|e| format!("Image illisible : {e}"))?;
        if bytes.len() > 4 * MAX_IMAGE_BYTES {
            return Err("Image trop lourde (48 Mo au maximum).".into());
        }
        let ext = image_ext(&bytes).ok_or("Format non pris en charge : choisissez une image JPEG, PNG, WebP ou AVIF.")?;
        let target = store.file_for(id, &format!("custom-{kind}-{}", util::now_secs()), ext);
        std::fs::write(&target, &bytes).map_err(|e| format!("Enregistrement impossible : {e}"))?;
        *slot = Some(util::path_string(&target));
    }
    store.set(id, entry);
    store.save();
    Ok(())
}
