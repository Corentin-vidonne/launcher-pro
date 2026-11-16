//! Battle.net : `product.db` (protobuf) recense les produits installés.
//!
//! Structure utile : Database { repeated ProductInstall = 1 } ;
//! ProductInstall { uid = 1, product_code = 2, settings = 3 { install_path = 1 } }.

use std::fs;
use std::path::{Path, PathBuf};

use super::{Ctx, Detected};
use crate::model::{Game, Launch};
use crate::util;

/// code produit → (nom affiché, code de lancement de Battle.net)
const PRODUCTS: &[(&str, &str, &str)] = &[
    ("wow", "World of Warcraft", "WoW"),
    ("wow_classic", "World of Warcraft Classic", "WoWC"),
    ("d3", "Diablo III", "D3"),
    ("osi", "Diablo II: Resurrected", "OSI"),
    ("fenris", "Diablo IV", "Fen"),
    ("anbs", "Diablo Immortal", "ANBS"),
    ("s1", "StarCraft", "S1"),
    ("s2", "StarCraft II", "S2"),
    ("w3", "Warcraft III: Reforged", "W3"),
    ("hero", "Heroes of the Storm", "Hero"),
    ("hsb", "Hearthstone", "WTCG"),
    ("wtcg", "Hearthstone", "WTCG"),
    ("pro", "Overwatch", "Pro"),
    ("gryphon", "Warcraft Rumble", "GRY"),
    ("viper", "Call of Duty: Black Ops 4", "VIPR"),
    ("odin", "Call of Duty: Modern Warfare", "ODIN"),
    ("lazarus", "Call of Duty: Modern Warfare 2 Campaign Remastered", "LAZR"),
    ("zeus", "Call of Duty: Black Ops Cold War", "ZEUS"),
    ("fore", "Call of Duty: Vanguard", "FORE"),
    ("auks", "Call of Duty", "AUKS"),
    ("wlby", "Crash Bandicoot 4: It's About Time", "WLBY"),
    ("rtro", "Blizzard Arcade Collection", "RTRO"),
];

pub fn scan(ctx: &Ctx) -> Vec<Detected> {
    let db = util::program_data().join("Battle.net").join("Agent").join("product.db");
    let products = fs::read(&db).map(|b| parse_products(&b)).unwrap_or_default();

    let client_dir = products
        .iter()
        .find(|p| p.code == "bna" || p.uid == "battle.net")
        .map(|p| PathBuf::from(&p.path))
        .or_else(|| {
            ctx.uninstall
                .iter()
                .find(|e| e.key == "Battle.net")
                .and_then(|e| e.location.clone())
        });
    let client = client_dir.map(|d| d.join("Battle.net.exe")).filter(|p| p.is_file());
    if client.is_none() && products.is_empty() {
        return Vec::new();
    }

    let mut d = Detected::new("battlenet", "Battle.net");
    if let Some(exe) = &client {
        d = d.with_launcher_exe(exe);
    }
    for p in products {
        if matches!(p.code.as_str(), "agent" | "bna") || p.path.is_empty() {
            continue;
        }
        let dir = PathBuf::from(p.path.replace('/', "\\"));
        if !dir.is_dir() {
            continue;
        }
        let known = PRODUCTS.iter().find(|(code, _, _)| *code == p.code);
        let name = known
            .map(|(_, name, _)| name.to_string())
            .or_else(|| dir.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| p.code.clone());
        let launch_code = known.map(|(_, _, c)| c.to_string()).unwrap_or_else(|| p.code.to_uppercase());

        let mut g = Game::new("battlenet", &p.code, name);
        g.install_dir = Some(util::path_string(&dir));
        if let Some(exe) = &client {
            g.launch = Launch::exe_args(util::path_string(exe), format!("--exec=\"launch {launch_code}\""));
        }
        g.art.icon_file = largest_exe(&dir).map(|p| util::path_string(&p));
        d.games.push(g);
    }
    vec![d]
}

/// Exécutable principal probable d'un dossier (le plus gros `.exe` à la racine), pour l'icône.
fn largest_exe(dir: &Path) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.path().extension().map(|x| x.eq_ignore_ascii_case("exe")).unwrap_or(false))
        .max_by_key(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
        .map(|e| e.path())
}

struct Product {
    uid: String,
    code: String,
    path: String,
}

fn parse_products(buf: &[u8]) -> Vec<Product> {
    let mut out = Vec::new();
    for (field, payload) in fields(buf) {
        if field != 1 {
            continue;
        }
        let mut p = Product { uid: String::new(), code: String::new(), path: String::new() };
        for (f, data) in fields(payload) {
            match f {
                1 => p.uid = text(data),
                2 => p.code = text(data),
                3 => {
                    if let Some((_, path)) = fields(data).into_iter().find(|(f, _)| *f == 1) {
                        p.path = text(path);
                    }
                }
                _ => {}
            }
        }
        if !p.code.is_empty() {
            out.push(p);
        }
    }
    out
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

/// Champs « length-delimited » d'un message protobuf (les autres types sont sautés).
fn fields(buf: &[u8]) -> Vec<(u64, &[u8])> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos < buf.len() {
        let Some(tag) = varint(buf, &mut pos) else { break };
        let field = tag >> 3;
        match tag & 7 {
            0 => {
                if varint(buf, &mut pos).is_none() {
                    break;
                }
            }
            1 => pos += 8,
            5 => pos += 4,
            2 => {
                let Some(len) = varint(buf, &mut pos) else { break };
                let Some(end) = pos.checked_add(len as usize).filter(|e| *e <= buf.len()) else { break };
                out.push((field, &buf[pos..end]));
                pos = end;
            }
            _ => break,
        }
    }
    out
}

fn varint(buf: &[u8], pos: &mut usize) -> Option<u64> {
    let mut value = 0u64;
    for shift in (0..64).step_by(7) {
        let byte = *buf.get(*pos)?;
        *pos += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some(value);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_product_install() {
        // ProductInstall { uid: "wow", product_code: "wow", settings { install_path: "C:/WoW" } }
        let settings = [&[0x0a, 6][..], b"C:/WoW"].concat();
        let install = [&[0x0a, 3][..], b"wow", &[0x12, 3], b"wow", &[0x1a, settings.len() as u8], &settings].concat();
        let db = [&[0x0a, install.len() as u8][..], &install].concat();
        let products = parse_products(&db);
        assert_eq!(products.len(), 1);
        assert_eq!(products[0].code, "wow");
        assert_eq!(products[0].path, "C:/WoW");
    }
}
