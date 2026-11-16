//! Détection automatique des launchers et des jeux installés.
//!
//! Chaque détecteur lit uniquement des données locales (registre, manifestes) : aucun accès réseau,
//! aucun compte requis. Ils tournent en parallèle ; l'ensemble prend quelques dizaines de ms.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::model::{CustomGame, Game, Launch, Library, Source};
use crate::util::{self, UninstallEntry};

mod battlenet;
mod ea;
mod epic;
mod gog;
mod hoyoplay;
mod other;
mod riot;
mod rockstar;
mod standalone;
mod steam;
mod ubisoft;
mod wargaming;
mod xbox;

/// Contexte partagé par les détecteurs (lu une seule fois).
pub struct Ctx {
    pub uninstall: Vec<UninstallEntry>,
}

/// Résultat d'un détecteur.
pub struct Detected {
    pub source: &'static str,
    pub name: &'static str,
    pub installed: bool,
    pub launcher: Launch,
    pub icon_file: Option<PathBuf>,
    pub games: Vec<Game>,
}

impl Detected {
    pub fn new(source: &'static str, name: &'static str) -> Self {
        Detected {
            source,
            name,
            installed: false,
            launcher: Launch::None,
            icon_file: None,
            games: Vec::new(),
        }
    }

    /// Déclare le launcher présent, avec son exécutable (sert aussi d'icône).
    pub fn with_launcher_exe(mut self, exe: &Path) -> Self {
        self.installed = true;
        self.launcher = Launch::exe(util::path_string(exe));
        self.icon_file = Some(exe.to_path_buf());
        self
    }
}

type Detector = fn(&Ctx) -> Vec<Detected>;

/// Ordre = priorité d'affichage et de dédoublonnage.
const DETECTORS: &[Detector] = &[
    steam::scan,
    epic::scan,
    ea::scan,
    ubisoft::scan,
    gog::scan,
    xbox::scan,
    battlenet::scan,
    riot::scan,
    rockstar::scan,
    hoyoplay::scan,
    wargaming::scan,
    standalone::scan,
];

pub fn scan_all(custom: &[CustomGame]) -> Library {
    let ctx = Ctx { uninstall: util::uninstall_entries() };

    // Un thread par détecteur : ils sont dominés par les E/S (registre + petits fichiers).
    let mut results: Vec<Detected> = std::thread::scope(|scope| {
        let handles: Vec<_> = DETECTORS
            .iter()
            .map(|detector| {
                let ctx = &ctx;
                scope.spawn(move || detector(ctx))
            })
            .collect();
        handles
            .into_iter()
            // Un détecteur qui panique n'emporte pas les autres.
            .flat_map(|h| h.join().unwrap_or_default())
            .collect()
    });

    // Jeux autonomes : passe heuristique, une fois connus les dossiers déjà attribués.
    let mut claimed: Vec<String> = Vec::new();
    for d in &results {
        for g in &d.games {
            if let Some(dir) = &g.install_dir {
                claimed.push(util::path_key(Path::new(dir)));
            }
        }
        if let Launch::Exe { path, .. } = &d.launcher {
            if let Some(parent) = Path::new(path).parent() {
                claimed.push(util::path_key(parent));
            }
        }
    }
    results.push(other::scan(&ctx, &claimed));
    results.push(custom_source(custom));

    let mut library = Library { scanned_at: util::now_secs(), ..Default::default() };
    let mut seen_ids = HashSet::new();
    let mut seen_dirs = HashSet::new();
    for d in results {
        let has_games = !d.games.is_empty();
        if d.installed || has_games {
            library.sources.push(Source {
                id: d.source.to_string(),
                name: d.name.to_string(),
                installed: d.installed,
                launcher: d.launcher,
                icon_file: d.icon_file.map(|p| util::path_string(&p)),
            });
        }
        for game in d.games {
            if !seen_ids.insert(game.id.clone()) {
                continue;
            }
            // Même dossier vu par deux sources (ex. jeu EA installé via Steam) : la première gagne.
            if let Some(dir) = &game.install_dir {
                if !seen_dirs.insert(util::path_key(Path::new(dir))) {
                    continue;
                }
            }
            library.games.push(game);
        }
    }
    library
        .games
        .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    library
}

fn custom_source(custom: &[CustomGame]) -> Detected {
    let mut d = Detected::new("custom", "Ajoutés manuellement");
    for c in custom {
        let exe = Path::new(&c.exe);
        if !exe.is_file() {
            continue;
        }
        let mut g = Game {
            id: c.id.clone(),
            ..Game::new("custom", "", c.name.clone())
        };
        g.install_dir = exe.parent().map(util::path_string);
        g.launch = Launch::Exe {
            path: c.exe.clone(),
            args: c.args.clone(),
            cwd: g.install_dir.clone(),
        };
        g.art.icon_file = Some(c.exe.clone());
        d.games.push(g);
    }
    d
}

/// Nom d'affichage tiré d'une entrée de désinstallation pointant sur `dir`.
pub(crate) fn uninstall_name_for_dir(ctx: &Ctx, dir: &Path) -> Option<String> {
    let key = util::path_key(dir);
    ctx.uninstall
        .iter()
        .find(|e| e.location.as_deref().map(util::path_key).as_deref() == Some(key.as_str()))
        .map(|e| e.name.clone())
}

/// Vrai si `path` est dans une bibliothèque Steam (jeu tiers installé via Steam).
pub(crate) fn is_under_steam(path: &Path) -> bool {
    util::path_key(path).contains("\\steamapps\\common")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Détection réelle sur la machine courante :
    /// `cargo test dump_scan -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn dump_scan() {
        let started = std::time::Instant::now();
        let library = scan_all(&[]);
        let elapsed = started.elapsed();
        for s in &library.sources {
            let count = library.games.iter().filter(|g| g.source == s.id).count();
            println!("[source] {:<10} {:<24} installé={} jeux={count} icône={:?}", s.id, s.name, s.installed, s.icon_file);
        }
        for g in &library.games {
            println!(
                "[jeu] {:<10} {:<52} tool={} cover={} hero={} icon={} launch={:?}",
                g.source,
                g.name,
                g.is_tool,
                g.art.cover_file.is_some() || g.art.cover_url.is_some(),
                g.art.hero_file.is_some() || g.art.hero_url.is_some(),
                g.art.icon_file.is_some(),
                g.launch
            );
        }
        println!("{} jeux, {} sources en {elapsed:?}", library.games.len(), library.sources.len());
        assert!(!library.sources.is_empty());
    }
}
