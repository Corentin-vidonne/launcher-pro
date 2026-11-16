//! Recherche rapide : petite fenêtre flottante (verre acrylique) appelée par un raccourci global,
//! n'importe où dans Windows. Créée à la première utilisation et libérée après un long moment sans
//! servir : elle ne coûte rien tant qu'on ne l'appelle pas.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use tauri::window::{Effect, EffectsBuilder};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

const LABEL: &str = "quick";
/// Fenêtre détruite après ce délai passé cachée : la mémoire de sa vue web est rendue.
const IDLE: Duration = Duration::from_secs(10 * 60);
/// Incrémenté à chaque ouverture : annule une destruction programmée devenue caduque.
static GENERATION: AtomicU64 = AtomicU64::new(0);
/// Le raccourci est bien enregistré auprès de Windows (il peut être pris par une autre application).
static ACTIVE: AtomicBool = AtomicBool::new(false);

/// Raccourcis de repli, essayés au démarrage si celui choisi est déjà pris ailleurs.
pub const FALLBACKS: &[&str] = &["alt+shift+space", "ctrl+shift+space"];

pub fn is_active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

/// (Re)déclare le raccourci global. Chaîne vide = désactivé.
pub fn apply_shortcut(app: &AppHandle, accelerator: &str) -> Result<(), String> {
    let shortcuts = app.global_shortcut();
    let _ = shortcuts.unregister_all();
    ACTIVE.store(false, Ordering::Relaxed);
    let accelerator = accelerator.trim();
    if accelerator.is_empty() {
        return Ok(());
    }
    let shortcut: Shortcut = accelerator
        .parse()
        .map_err(|e| format!("Raccourci invalide ({accelerator}) : {e}"))?;
    shortcuts
        .register(shortcut)
        .map_err(|_| "Ce raccourci est déjà utilisé par une autre application.".to_string())?;
    ACTIVE.store(true, Ordering::Relaxed);
    Ok(())
}

/// Journal de diagnostic : les erreurs toujours, le reste seulement avec LAUNCHERPRO_DEBUG=1.
fn log(app: &AppHandle, message: &str, always: bool) {
    if !always && std::env::var_os("LAUNCHERPRO_DEBUG").is_none() {
        return;
    }
    if let Ok(dir) = app.path().app_data_dir() {
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("quick.log")) {
            let _ = writeln!(file, "[{}] {message}", crate::util::now_secs());
        }
    }
}

/// Appui sur le raccourci : ouvre la recherche, ou la referme si elle est déjà affichée.
pub fn toggle(app: &AppHandle) {
    log(app, "raccourci reçu", false);
    match app.get_webview_window(LABEL) {
        Some(window) if window.is_visible().unwrap_or(false) => hide(app),
        Some(window) => show(&window),
        None => {
            GENERATION.fetch_add(1, Ordering::Relaxed);
            // Sous Windows, créer une vue web depuis un gestionnaire d'évènement synchrone peut
            // bloquer : la création se fait dans une tâche à part.
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                match build(&app) {
                    Ok(window) => {
                        round_corners(&window);
                        place(&window);
                        log(&app, "fenêtre créée", false);
                        // La page s'affiche elle-même une fois prête : pas d'éclair blanc.
                    }
                    Err(e) => log(&app, &format!("création de la fenêtre impossible : {e}"), true),
                }
            });
        }
    }
}

pub fn hide(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else { return };
    let _ = window.hide();
    let generation = GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(IDLE).await;
        if GENERATION.load(Ordering::Relaxed) != generation {
            return; // rouverte entre-temps
        }
        if let Some(window) = app.get_webview_window(LABEL) {
            if !window.is_visible().unwrap_or(true) {
                let _ = window.destroy();
            }
        }
    });
}

fn show(window: &WebviewWindow) {
    GENERATION.fetch_add(1, Ordering::Relaxed);
    place(window);
    let _ = window.emit_to(LABEL, "quick-open", ());
    let _ = window.show();
    let _ = window.set_focus();
}

fn build(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("quick.html".into()))
        .title("Recherche rapide")
        .inner_size(680.0, 136.0)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .effects(EffectsBuilder::new().effect(Effect::Acrylic).build())
        .build()
}

/// Centrée horizontalement, au premier quart de l'écran où se trouve la souris.
fn place(window: &WebviewWindow) {
    let monitors = window.available_monitors().unwrap_or_default();
    let cursor = window.cursor_position().ok();
    let monitor = cursor
        .and_then(|c| {
            monitors.into_iter().find(|m| {
                let (p, s) = (m.position(), m.size());
                c.x >= p.x as f64
                    && c.x < (p.x + s.width as i32) as f64
                    && c.y >= p.y as f64
                    && c.y < (p.y + s.height as i32) as f64
            })
        })
        .or_else(|| window.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else {
        let _ = window.center();
        return;
    };
    let (origin, area) = (monitor.position(), monitor.size());
    let width = window.outer_size().map(|s| s.width).unwrap_or(680) as i32;
    let x = origin.x + (area.width as i32 - width) / 2;
    let y = origin.y + (area.height as f64 * 0.2) as i32;
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

/// Coins arrondis façon Windows 11 pour la fenêtre sans bordure.
fn round_corners(window: &WebviewWindow) {
    use std::ffi::c_void;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND};

    let Ok(hwnd) = window.hwnd() else { return };
    let preference = DWMWCP_ROUND;
    unsafe {
        let _ = DwmSetWindowAttribute(
            HWND(hwnd.0 as _),
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &preference as *const _ as *const c_void,
            std::mem::size_of_val(&preference) as u32,
        );
    }
}

/// Ramène la fenêtre principale au premier plan ; avec une recherche, ouvre le magasin dessus.
pub fn show_main(app: &AppHandle, store_query: Option<String>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
    if let Some(query) = store_query {
        let _ = app.emit_to("main", "open-store", query);
    }
}

/// Bouton « Essayer » des réglages : ouvre la recherche rapide comme le raccourci.
#[tauri::command]
pub fn quick_open(app: AppHandle) {
    toggle(&app);
}

#[tauri::command]
pub fn quick_hide(app: AppHandle) {
    hide(&app);
}

#[tauri::command]
pub fn quick_store(app: AppHandle, query: String) {
    hide(&app);
    show_main(&app, Some(query));
}
