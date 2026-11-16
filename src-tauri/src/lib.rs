//! Launcher Pro — backend.
//!
//! - `library` : détection des launchers et jeux installés (lecture locale uniquement)
//! - `art`     : jaquettes (cache disque) servies par le protocole `art`
//! - `store`   : recherche et comparaison de prix entre boutiques
//! - `shell`   : lancement des jeux, extraction d'icônes

mod art;
mod commands;
mod library;
mod model;
mod quick;
mod shell;
mod startup;
mod state;
mod storage;
mod store;
mod util;
mod vault;

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{Manager, WindowEvent};

use state::AppState;

/// Dernier état appliqué, pour ne pas rappeler WebView2 à chaque pixel de redimensionnement.
static LOW_MEMORY: AtomicBool = AtomicBool::new(false);

/// Fenêtre réduite (typiquement pendant une partie) : WebView2 passe en cible mémoire basse et
/// libère l'essentiel de sa mémoire vive ; retour à la normale dès que la fenêtre réapparaît.
fn set_low_memory(window: &tauri::Window, low: bool) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
    };
    use windows::core::Interface;

    if LOW_MEMORY.swap(low, Ordering::Relaxed) == low {
        return;
    }
    let Some(webview) = window.app_handle().get_webview_window(window.label()) else { return };
    let _ = webview.with_webview(move |platform| unsafe {
        let level = if low {
            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
        } else {
            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
        };
        if let Ok(core) = platform.controller().CoreWebView2() {
            if let Ok(core) = core.cast::<ICoreWebView2_19>() {
                let _ = core.SetMemoryUsageTargetLevel(level);
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Une seule instance : relancer l'app (raccourci, menu Démarrer) ramène la fenêtre existante.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| quick::show_main(app, None)))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![startup::AUTOSTART_ARG]),
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        quick::toggle(app);
                    }
                })
                .build(),
        )
        .register_asynchronous_uri_scheme_protocol("art", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            tauri::async_runtime::spawn(async move {
                responder.respond(art::serve(&app, request.uri().path()).await);
            });
        })
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let cache_dir = app.path().app_cache_dir()?;
            // La bibliothèque du dernier lancement est disponible immédiatement ;
            // le rescan se fait en arrière-plan et met l'interface à jour à son arrivée.
            app.manage(AppState::init(data_dir, cache_dir));
            // Raccourci global de la recherche rapide. S'il est déjà pris par une autre application,
            // on se rabat sur un raccourci libre (et on le retient) ; sinon l'app démarre sans.
            let state = app.state::<AppState>();
            // Démarrage avec Windows : la clé suit le réglage (et le chemin de l'exécutable actuel).
            let launch_at_startup = state.prefs_read().settings.launch_at_startup;
            if let Err(e) = startup::apply(app.handle(), launch_at_startup) {
                eprintln!("{e}");
            }
            // Lancé par Windows à l'ouverture de session : fenêtre agrandie.
            // Elle est encore masquée ; l'interface l'affiche une fois prête, comme d'habitude.
            if std::env::args().any(|a| a == startup::AUTOSTART_ARG) {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.maximize();
                }
            }
            let wanted = state.prefs_read().settings.quick_shortcut.clone();
            if !wanted.is_empty() && quick::apply_shortcut(app.handle(), &wanted).is_err() {
                for fallback in quick::FALLBACKS.iter().filter(|f| **f != wanted) {
                    if quick::apply_shortcut(app.handle(), fallback).is_ok() {
                        state.prefs_write().settings.quick_shortcut = fallback.to_string();
                        state.save_prefs();
                        break;
                    }
                }
            }
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                state::refresh(handle).await;
            });
            // La fenêtre démarre masquée et c'est l'interface qui la révèle une fois prête.
            // Filet de sécurité : si l'interface n'y parvient pas, on l'affiche quand même.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                if let Some(window) = handle.get_webview_window("main") {
                    let _ = window.show();
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, WindowEvent::Resized(_)) {
                set_low_memory(window, window.is_minimized().unwrap_or(false));
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_library,
            commands::rescan_library,
            commands::launch_game,
            commands::open_source,
            commands::reveal_game,
            commands::set_favorite,
            commands::set_hidden,
            commands::vault_status,
            commands::vault_set_password,
            commands::vault_unlock,
            commands::vault_lock,
            commands::add_custom_game,
            commands::remove_custom_game,
            commands::get_app_info,
            commands::set_settings,
            commands::store_search,
            commands::store_details,
            commands::store_offers,
            commands::store_featured,
            commands::open_url,
            commands::set_custom_art,
            commands::get_collections,
            commands::save_collection,
            commands::delete_collection,
            commands::toggle_in_collection,
            storage::storage_info,
            quick::quick_open,
            quick::quick_hide,
            quick::quick_store,
        ])
        .run(tauri::generate_context!())
        .expect("erreur au démarrage de Launcher Pro");
}
