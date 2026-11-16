//! Riot Client (VALORANT, League of Legends…) : métadonnées YAML dans ProgramData.

use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use super::{Ctx, Detected};
use crate::model::{Game, Launch};
use crate::util;

fn product_name(product: &str) -> Option<&'static str> {
    Some(match product {
        "valorant" => "VALORANT",
        "league_of_legends" => "League of Legends",
        "bacon" => "Legends of Runeterra",
        "lion" => "2XKO",
        _ => return None,
    })
}

pub fn scan(_ctx: &Ctx) -> Vec<Detected> {
    let base = util::program_data().join("Riot Games");
    let installs = fs::read_to_string(base.join("RiotClientInstalls.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok());
    let client = installs
        .as_ref()
        .and_then(|v| {
            ["rc_default", "rc_live", "rc_beta"]
                .iter()
                .find_map(|k| v.get(*k).and_then(Value::as_str))
        })
        .map(|p| PathBuf::from(p.replace('/', "\\")))
        .filter(|p| p.is_file());
    let Some(client) = client else { return Vec::new() };
    let mut d = Detected::new("riot", "Riot Games").with_launcher_exe(&client);

    // Metadata\<produit>.<canal>\<produit>.<canal>.product_settings.yaml
    for entry in fs::read_dir(base.join("Metadata")).into_iter().flatten().flatten() {
        let folder = entry.file_name().to_string_lossy().to_string();
        let Some((product, patchline)) = folder.split_once('.') else { continue };
        let Some(name) = product_name(product) else { continue };
        let settings = entry.path().join(format!("{folder}.product_settings.yaml"));
        let Ok(yaml) = fs::read_to_string(settings) else { continue };
        let dir = yaml
            .lines()
            .find_map(|l| l.trim().strip_prefix("product_install_full_path:"))
            .map(|v| PathBuf::from(v.trim().trim_matches('"').replace('/', "\\")));
        let Some(dir) = dir.filter(|p| p.is_dir()) else { continue };

        let mut g = Game::new("riot", &folder, name);
        g.install_dir = Some(util::path_string(&dir));
        g.launch = Launch::exe_args(
            util::path_string(&client),
            format!("--launch-product={product} --launch-patchline={patchline}"),
        );
        d.games.push(g);
    }
    vec![d]
}
