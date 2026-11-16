//! Commandes appelées par l'interface.

use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, State};

use crate::model::{Collection, CustomGame, GameDto, LibraryDto, Settings};
use crate::state::{self, AppState};
use crate::{art, quick, shell, store, util, vault};

const STORE_TTL: Duration = Duration::from_secs(10 * 60);

// ───────────────────────────── Bibliothèque ─────────────────────────────

#[tauri::command]
pub fn get_library(state: State<'_, AppState>) -> LibraryDto {
    state.library_dto()
}

#[tauri::command]
pub async fn rescan_library(app: AppHandle) -> LibraryDto {
    state::refresh(app).await
}

#[tauri::command]
pub fn launch_game(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let game = state.find_game(&id).ok_or("Jeu introuvable.")?;
    shell::launch(&game.launch)?;
    state::mark_launched(&state, &id);
    Ok(())
}

#[tauri::command]
pub fn open_source(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let launcher = state
        .library_read()
        .sources
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.launcher.clone())
        .ok_or("Launcher introuvable.")?;
    shell::launch(&launcher)
}

#[tauri::command]
pub fn reveal_game(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let dir = state
        .find_game(&id)
        .and_then(|g| g.install_dir)
        .filter(|d| Path::new(d).is_dir())
        .ok_or("Dossier d'installation introuvable.")?;
    shell::open(&dir, None, None)
}

#[tauri::command]
pub fn set_favorite(state: State<'_, AppState>, id: String, value: bool) {
    {
        let mut prefs = state.prefs_write();
        if value {
            prefs.favorites.insert(id);
        } else {
            prefs.favorites.remove(&id);
        }
    }
    state.save_prefs();
}

#[tauri::command]
pub fn set_hidden(state: State<'_, AppState>, id: String, value: bool) -> Result<LibraryDto, String> {
    // Masquer est toujours possible ; réafficher n'est possible que depuis le coffre déverrouillé.
    if !value && !state.hidden_unlocked.load(Ordering::Relaxed) {
        return Err("Déverrouillez les jeux masqués dans les réglages pour réafficher un jeu.".into());
    }
    {
        let mut prefs = state.prefs_write();
        if value {
            prefs.hidden.insert(id);
        } else {
            prefs.hidden.remove(&id);
        }
    }
    state.save_prefs();
    Ok(state.library_dto())
}

// ───────────────────────────── Coffre des jeux masqués ─────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatus {
    has_password: bool,
    unlocked: bool,
}

#[tauri::command]
pub fn vault_status(state: State<'_, AppState>) -> VaultStatus {
    VaultStatus {
        has_password: state.prefs_read().hidden_password.is_some(),
        unlocked: state.hidden_unlocked.load(Ordering::Relaxed),
    }
}

/// Après un échec, une courte pause rend inutile l'essai de mots de passe à la chaîne.
async fn refuse(message: &str) -> Result<LibraryDto, String> {
    tokio::time::sleep(Duration::from_millis(700)).await;
    Err(message.to_string())
}

/// Définit (ou change, avec l'actuel) le mot de passe, puis ouvre le coffre.
/// Renvoie la bibliothèque, jeux masqués compris : l'onglet « Masqués » apparaît.
#[tauri::command]
pub async fn vault_set_password(
    state: State<'_, AppState>,
    password: String,
    current: Option<String>,
) -> Result<LibraryDto, String> {
    if password.chars().count() < vault::MIN_LEN {
        return Err(format!("Le mot de passe doit contenir au moins {} caractères.", vault::MIN_LEN));
    }
    let existing = state.prefs_read().hidden_password.clone();
    if let Some(hash) = existing {
        if !vault::verify(current.as_deref().unwrap_or(""), &hash) {
            return refuse("Mot de passe actuel incorrect.").await;
        }
    }
    let hash = vault::hash(&password)?;
    state.prefs_write().hidden_password = Some(hash);
    state.save_prefs();
    state.hidden_unlocked.store(true, Ordering::Relaxed);
    Ok(state.library_dto())
}

#[tauri::command]
pub async fn vault_unlock(state: State<'_, AppState>, password: String) -> Result<LibraryDto, String> {
    let existing = state.prefs_read().hidden_password.clone();
    let Some(hash) = existing else {
        return Err("Aucun mot de passe n'est encore défini.".into());
    };
    if !vault::verify(&password, &hash) {
        return refuse("Mot de passe incorrect.").await;
    }
    state.hidden_unlocked.store(true, Ordering::Relaxed);
    Ok(state.library_dto())
}

/// Referme le coffre : la bibliothèque renvoyée ne contient plus les jeux masqués.
#[tauri::command]
pub fn vault_lock(state: State<'_, AppState>) -> LibraryDto {
    state.hidden_unlocked.store(false, Ordering::Relaxed);
    state.library_dto()
}

#[tauri::command]
pub async fn add_custom_game(app: AppHandle, state: State<'_, AppState>, path: String, name: Option<String>) -> Result<LibraryDto, String> {
    let exe = Path::new(&path);
    if !exe.is_file() {
        return Err("Fichier introuvable.".into());
    }
    let fallback = exe.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let name = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).unwrap_or(fallback);
    let id = format!("custom:{}", util::path_key(exe));
    {
        let mut prefs = state.prefs_write();
        prefs.custom.retain(|c| c.id != id);
        prefs.custom.push(CustomGame { id, name, exe: util::path_string(exe), args: None });
    }
    state.save_prefs();
    Ok(state::refresh(app).await)
}

#[tauri::command]
pub async fn remove_custom_game(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<LibraryDto, String> {
    state.prefs_write().custom.retain(|c| c.id != id);
    state.save_prefs();
    Ok(state::refresh(app).await)
}

// ───────────────────────────── Réglages ─────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    settings: Settings,
    /// Effet Mica disponible (Windows 11) : sinon l'interface utilise des fonds opaques.
    translucent: bool,
    country: String,
    version: &'static str,
    /// Le raccourci de recherche rapide fonctionne réellement (pas pris par une autre app).
    quick_shortcut_active: bool,
}

#[tauri::command]
pub fn get_app_info(state: State<'_, AppState>) -> AppInfo {
    let build = util::hklm_string(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", "CurrentBuildNumber")
        .and_then(|b| b.parse::<u32>().ok())
        .unwrap_or(0);
    AppInfo {
        settings: state.prefs_read().settings.clone(),
        translucent: build >= 22000,
        country: state.region.country.clone(),
        version: env!("CARGO_PKG_VERSION"),
        quick_shortcut_active: quick::is_active(),
    }
}

#[tauri::command]
pub fn set_settings(app: AppHandle, state: State<'_, AppState>, mut settings: Settings) -> Result<(), String> {
    // Nouveau raccourci de recherche rapide : s'il est déjà pris, on garde l'ancien.
    let previous = state.prefs_read().settings.quick_shortcut.clone();
    let mut outcome = Ok(());
    if settings.quick_shortcut != previous {
        if let Err(e) = quick::apply_shortcut(&app, &settings.quick_shortcut) {
            let _ = quick::apply_shortcut(&app, &previous);
            settings.quick_shortcut = previous;
            outcome = Err(e);
        }
    }
    state.prefs_write().settings = settings;
    state.save_prefs();
    // Les offres affichées dépendent des réglages.
    state.store_cache.lock().unwrap_or_else(|e| e.into_inner()).clear();
    outcome
}

// ───────────────────────────── Magasin ─────────────────────────────

fn cache_get(state: &AppState, key: &str) -> Option<Value> {
    let cache = state.store_cache.lock().unwrap_or_else(|e| e.into_inner());
    cache
        .get(key)
        .filter(|(at, _)| at.elapsed() < STORE_TTL)
        .map(|(_, v)| v.clone())
}

fn cache_put(state: &AppState, key: String, value: &Value) {
    let mut cache = state.store_cache.lock().unwrap_or_else(|e| e.into_inner());
    cache.retain(|_, (at, _)| at.elapsed() < STORE_TTL);
    cache.insert(key, (Instant::now(), value.clone()));
}

/// Boutiques à afficher : celles dont le launcher est sur le PC, plus les revendeurs de clés.
fn store_filter(state: &AppState) -> impl Fn(&str) -> bool {
    let installed: HashSet<String> = state
        .library_read()
        .sources
        .iter()
        .filter(|s| s.installed)
        .map(|s| s.id.clone())
        .collect();
    let show_all = state.prefs_read().settings.show_all_stores;
    move |store_id: &str| match store::store_source(store_id) {
        None => true,
        Some(source) => show_all || installed.contains(source),
    }
}

async fn search_cached(state: &AppState, query: &str) -> Result<Value, String> {
    let key = format!("search:{}", util::norm_title(query));
    if let Some(hit) = cache_get(state, &key) {
        return Ok(hit);
    }
    let result = store::search(&state.http, &state.region, query, store_filter(state)).await;
    let value = serde_json::to_value(&result).map_err(|e| e.to_string())?;
    // Une recherche où toutes les boutiques ont échoué (hors ligne) n'est pas mise en cache.
    if !result.hits.is_empty() {
        cache_put(state, key, &value);
    }
    Ok(value)
}

#[tauri::command]
pub async fn store_search(state: State<'_, AppState>, query: String) -> Result<Value, String> {
    let query = query.trim();
    if query.chars().count() < 2 {
        return Ok(serde_json::json!({ "hits": [], "failed": [] }));
    }
    search_cached(&state, query).await
}

#[tauri::command]
pub async fn store_details(state: State<'_, AppState>, appid: u32) -> Result<Value, String> {
    let key = format!("details:{appid}");
    if let Some(hit) = cache_get(&state, &key) {
        return Ok(hit);
    }
    let details = store::steam::details(&state.http, &state.region, appid).await?;
    let value = serde_json::to_value(details).map_err(|e| e.to_string())?;
    cache_put(&state, key, &value);
    Ok(value)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LinkStore {
    source: String,
    name: String,
    /// Page de recherche de la boutique ; sans elle, l'interface ouvre le launcher.
    url: Option<String>,
}

/// Launchers sans API de prix publique : (source, mots-clés éditeur, plateformes de clé associées).
const LINK_RULES: &[(&str, &[&str], &[&str])] = &[
    ("ea", &["electronic arts", "ea sports", "ea games", "respawn"], &["ea app", "origin"]),
    ("ubisoft", &["ubisoft"], &["ubisoft", "uplay"]),
    ("battlenet", &["blizzard", "activision"], &["battle.net"]),
    ("rockstar", &["rockstar"], &["rockstar"]),
];

/// Offres complètes pour un jeu précis : nouvelle recherche ciblée sur son titre exact,
/// plus les launchers installés susceptibles de le vendre sans prix consultable.
#[tauri::command]
pub async fn store_offers(state: State<'_, AppState>, title: String, publishers: Vec<String>) -> Result<Value, String> {
    let result = search_cached(&state, &title).await?;
    let key = util::match_key(&title);
    let offers = result["hits"]
        .as_array()
        .and_then(|hits| hits.iter().find(|h| h["key"] == key.as_str()))
        .map(|h| h["offers"].clone())
        .unwrap_or_else(|| Value::Array(Vec::new()));

    let publishers: Vec<String> = publishers.iter().map(|p| p.to_lowercase()).collect();
    let key_platforms: Vec<String> = offers
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|o| o["note"].as_str().map(str::to_lowercase))
        .collect();
    let links: Vec<LinkStore> = {
        let library = state.library_read();
        let installed = |id: &str| library.sources.iter().find(|s| s.id == id && s.installed);
        let mut links: Vec<LinkStore> = Vec::new();
        // Epic n'expose pas ses prix aux applications tierces : on renvoie vers sa page de recherche.
        if let Some(epic) = installed("epic") {
            let query = percent_encoding::utf8_percent_encode(&title, percent_encoding::NON_ALPHANUMERIC);
            links.push(LinkStore {
                source: epic.id.clone(),
                name: epic.name.clone(),
                url: Some(format!(
                    "https://store.epicgames.com/{}/browse?q={query}&sortBy=relevancy&sortDir=DESC",
                    state.region.lang
                )),
            });
        }
        links.extend(
            LINK_RULES
                .iter()
                .filter(|(_, pubs, platforms)| {
                    pubs.iter().any(|kw| publishers.iter().any(|p| p.contains(kw)))
                        || platforms.iter().any(|kw| key_platforms.iter().any(|n| n.contains(kw)))
                })
                .filter_map(|(source, _, _)| installed(source))
                .map(|s| LinkStore { source: s.id.clone(), name: s.name.clone(), url: None }),
        );
        links
    };
    Ok(serde_json::json!({ "offers": offers, "links": links, "failed": result["failed"] }))
}

#[tauri::command]
pub async fn store_featured(state: State<'_, AppState>) -> Result<Value, String> {
    let key = "featured".to_string();
    if let Some(hit) = cache_get(&state, &key) {
        return Ok(hit);
    }
    let featured = store::steam::featured(&state.http, &state.region).await?;
    let value = serde_json::to_value(featured).map_err(|e| e.to_string())?;
    cache_put(&state, key, &value);
    Ok(value)
}

/// Ouvre une page web dans le navigateur par défaut (liens http/https uniquement).
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("Lien non autorisé.".into());
    }
    shell::open(&url, None, None)
}

// ───────────────────────────── Jaquettes personnalisées ─────────────────────────────

/// `kind` = "cover" (jaquette) ou "hero" (fond) ; `path` absent = revenir à l'image automatique.
#[tauri::command]
pub fn set_custom_art(state: State<'_, AppState>, id: String, kind: String, path: Option<String>) -> Result<GameDto, String> {
    let game = state.find_game(&id).ok_or("Jeu introuvable.")?;
    art::set_custom(&state.art, &id, &kind, path.as_deref().map(Path::new))?;
    Ok(state.game_dto(&game))
}

// ───────────────────────────── Collections ─────────────────────────────

#[tauri::command]
pub fn get_collections(state: State<'_, AppState>) -> Vec<Collection> {
    state.prefs_read().collections.clone()
}

/// Crée une collection (`id` absent) ou la renomme.
#[tauri::command]
pub fn save_collection(state: State<'_, AppState>, id: Option<String>, name: String) -> Result<Vec<Collection>, String> {
    let name: String = name.trim().chars().take(40).collect();
    if name.is_empty() {
        return Err("Donnez un nom à la collection.".into());
    }
    {
        let mut prefs = state.prefs_write();
        match id {
            Some(id) => {
                let collection = prefs.collections.iter_mut().find(|c| c.id == id).ok_or("Collection introuvable.")?;
                collection.name = name;
            }
            None => {
                let nanos = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0);
                prefs.collections.push(Collection { id: format!("c{nanos:x}"), name, games: Vec::new() });
            }
        }
    }
    state.save_prefs();
    Ok(state.prefs_read().collections.clone())
}

/// Supprime le regroupement (les jeux eux-mêmes ne sont pas touchés).
#[tauri::command]
pub fn delete_collection(state: State<'_, AppState>, id: String) -> Vec<Collection> {
    state.prefs_write().collections.retain(|c| c.id != id);
    state.save_prefs();
    state.prefs_read().collections.clone()
}

/// Ajoute le jeu à la collection, ou l'en retire s'il y est déjà.
#[tauri::command]
pub fn toggle_in_collection(state: State<'_, AppState>, id: String, game_id: String) -> Result<Vec<Collection>, String> {
    {
        let mut prefs = state.prefs_write();
        let collection = prefs.collections.iter_mut().find(|c| c.id == id).ok_or("Collection introuvable.")?;
        if let Some(pos) = collection.games.iter().position(|g| *g == game_id) {
            collection.games.remove(pos);
        } else {
            collection.games.push(game_id);
        }
    }
    state.save_prefs();
    Ok(state.prefs_read().collections.clone())
}
