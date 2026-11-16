//! Modèle de données : jeux, sources, préférences.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

/// Manière de démarrer un jeu ou un launcher.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Launch {
    /// Protocole (`steam://…`, `com.epicgames.launcher://…`, `shell:AppsFolder\…`).
    Uri { uri: String },
    /// Exécutable avec arguments facultatifs.
    Exe {
        path: String,
        #[serde(default)]
        args: Option<String>,
        #[serde(default)]
        cwd: Option<String>,
    },
    #[default]
    None,
}

impl Launch {
    pub fn uri(uri: impl Into<String>) -> Self {
        Launch::Uri { uri: uri.into() }
    }

    pub fn exe(path: impl Into<String>) -> Self {
        Launch::Exe { path: path.into(), args: None, cwd: None }
    }

    pub fn exe_args(path: impl Into<String>, args: impl Into<String>) -> Self {
        Launch::Exe { path: path.into(), args: Some(args.into()), cwd: None }
    }

    pub fn is_some(&self) -> bool {
        !matches!(self, Launch::None)
    }
}

/// Indices pour retrouver les visuels d'un jeu, du plus fiable (fichier local) au moins fiable.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Art {
    #[serde(default)]
    pub cover_file: Option<String>,
    #[serde(default)]
    pub hero_file: Option<String>,
    #[serde(default)]
    pub logo_file: Option<String>,
    /// `.ico`/`.png` servi tel quel, ou `.exe` dont on extrait l'icône.
    #[serde(default)]
    pub icon_file: Option<String>,
    #[serde(default)]
    pub cover_url: Option<String>,
    #[serde(default)]
    pub hero_url: Option<String>,
    #[serde(default)]
    pub steam_appid: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub id: String,
    pub source: String,
    pub name: String,
    #[serde(default)]
    pub install_dir: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<u64>,
    #[serde(default)]
    pub last_played: Option<u64>,
    #[serde(default)]
    pub playtime_min: Option<u64>,
    #[serde(default)]
    pub is_tool: bool,
    #[serde(default)]
    pub launch: Launch,
    #[serde(default)]
    pub art: Art,
}

impl Game {
    pub fn new(source: &str, key: impl AsRef<str>, name: impl Into<String>) -> Self {
        Game {
            id: format!("{source}:{}", key.as_ref()),
            source: source.to_string(),
            name: name.into(),
            install_dir: None,
            size_bytes: None,
            last_played: None,
            playtime_min: None,
            is_tool: false,
            launch: Launch::None,
            art: Art::default(),
        }
    }
}

/// État d'une source (launcher) sur ce PC.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub name: String,
    /// Le launcher lui-même est présent.
    pub installed: bool,
    #[serde(default)]
    pub launcher: Launch,
    /// Exécutable ou icône dont on tire le pictogramme de la source.
    #[serde(default)]
    pub icon_file: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Library {
    #[serde(default)]
    pub games: Vec<Game>,
    #[serde(default)]
    pub sources: Vec<Source>,
    #[serde(default)]
    pub scanned_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomGame {
    pub id: String,
    pub name: String,
    pub exe: String,
    #[serde(default)]
    pub args: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// "system" | "dark" | "light"
    pub theme: String,
    /// Que devient Launcher Pro une fois un jeu lancé : "stay" | "minimize" | "quit".
    pub after_launch: String,
    pub show_tools: bool,
    /// Afficher aussi les boutiques dont le launcher n'est pas installé.
    pub show_all_stores: bool,
    /// "small" | "medium" | "large"
    pub grid_size: String,
    /// Verre translucide avec réfraction ; désactivé = panneaux opaques, sans coût GPU.
    pub liquid_glass: bool,
    /// Raccourci global de la recherche rapide (« ctrl+alt+space »…) ; vide = désactivé.
    pub quick_shortcut: String,
    /// Lancer Launcher Pro (agrandi) à l'ouverture de session Windows.
    pub launch_at_startup: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: "system".into(),
            after_launch: "minimize".into(),
            show_tools: false,
            show_all_stores: false,
            grid_size: "medium".into(),
            liquid_glass: true,
            quick_shortcut: "ctrl+alt+space".into(),
            launch_at_startup: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Prefs {
    pub favorites: HashSet<String>,
    pub hidden: HashSet<String>,
    pub custom: Vec<CustomGame>,
    /// Dernier lancement depuis Launcher Pro (secondes Unix), par id de jeu.
    pub launched: HashMap<String, u64>,
    pub settings: Settings,
    /// Empreinte Argon2 du mot de passe du coffre des jeux masqués (jamais le mot de passe).
    pub hidden_password: Option<String>,
    pub collections: Vec<Collection>,
}

/// Regroupement libre de jeux, façon playlist.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub games: Vec<String>,
}

// ───────────────────────────── DTO envoyés à l'interface ─────────────────────────────

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameDto {
    pub id: String,
    pub source: String,
    pub name: String,
    pub install_dir: Option<String>,
    pub size_bytes: Option<u64>,
    pub last_played: Option<u64>,
    pub playtime_min: Option<u64>,
    pub is_tool: bool,
    pub favorite: bool,
    pub hidden: bool,
    pub can_launch: bool,
    pub cover: Option<String>,
    pub hero: Option<String>,
    pub logo: Option<String>,
    pub icon: Option<String>,
    /// Jaquette ou fond choisi par l'utilisateur.
    pub custom_art: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceDto {
    pub id: String,
    pub name: String,
    pub installed: bool,
    pub can_open: bool,
    pub count: usize,
    pub icon: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDto {
    pub games: Vec<GameDto>,
    pub sources: Vec<SourceDto>,
    pub scanned_at: u64,
    /// Une analyse du PC est en cours (la liste peut encore changer).
    pub scanning: bool,
}
