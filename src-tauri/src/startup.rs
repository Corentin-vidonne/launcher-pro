//! Démarrage avec Windows : une entrée dans la clé « Run » de l'utilisateur lance Launcher Pro,
//! agrandi (`--autostart`), à l'ouverture de session. Aucun droit administrateur nécessaire.

use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

/// Argument passé par Windows au démarrage de session.
pub const AUTOSTART_ARG: &str = "--autostart";

/// Aligne la clé « Run » sur le réglage. Réactivée à chaque lancement quand le réglage est actif,
/// pour qu'elle pointe toujours vers l'exécutable en cours (après une mise à jour, par exemple).
pub fn apply(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let launcher = app.autolaunch();
    if enabled {
        launcher.enable().map_err(|e| format!("Démarrage avec Windows impossible : {e}"))
    } else if launcher.is_enabled().unwrap_or(false) {
        launcher.disable().map_err(|e| format!("Impossible de retirer le démarrage avec Windows : {e}"))
    } else {
        Ok(())
    }
}
