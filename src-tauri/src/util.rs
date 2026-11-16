//! Petits utilitaires partagés : registre Windows, chemins, parseur VDF, normalisation de titres.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use winreg::enums::*;
use winreg::RegKey;

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn env_path(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(PathBuf::from)
}

pub fn program_data() -> PathBuf {
    env_path("ProgramData").unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"))
}

/// Clé de comparaison insensible à la casse et au sens des barres obliques.
pub fn path_key(p: &Path) -> String {
    p.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

pub fn path_string(p: &Path) -> String {
    p.to_string_lossy().replace('/', "\\")
}

pub fn mtime_secs(p: &Path) -> u64 {
    std::fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ───────────────────────────── Registre ─────────────────────────────

/// Ouvre une clé HKLM dans les vues 64 et 32 bits (évite de coder WOW6432Node en dur).
pub fn hklm_views(path: &str) -> Vec<RegKey> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    [KEY_WOW64_64KEY, KEY_WOW64_32KEY]
        .iter()
        .filter_map(|view| hklm.open_subkey_with_flags(path, KEY_READ | *view).ok())
        .collect()
}

pub fn hkcu(path: &str) -> Option<RegKey> {
    RegKey::predef(HKEY_CURRENT_USER).open_subkey(path).ok()
}

pub fn reg_string(key: &RegKey, name: &str) -> Option<String> {
    key.get_value::<String, _>(name)
        .ok()
        .map(|s| s.trim().trim_matches('\0').to_string())
        .filter(|s| !s.is_empty())
}

pub fn reg_u32(key: &RegKey, name: &str) -> Option<u32> {
    key.get_value::<u32, _>(name).ok()
}

/// Première valeur chaîne trouvée dans HKLM (vue 64 puis 32 bits).
pub fn hklm_string(path: &str, name: &str) -> Option<String> {
    hklm_views(path).iter().find_map(|k| reg_string(k, name))
}

/// Nettoie une valeur du type `"C:\jeu\jeu.exe",0` en chemin exploitable.
pub fn clean_exe_path(raw: &str) -> Option<PathBuf> {
    let mut s = raw.trim();
    if let Some(rest) = s.strip_prefix('"') {
        s = rest.split('"').next().unwrap_or(rest);
    } else if let Some(idx) = s.rfind(',') {
        // suffixe d'index d'icône : `chemin,0`
        if s[idx + 1..].trim().parse::<i32>().is_ok() {
            s = &s[..idx];
        }
    }
    let s = s.trim().trim_matches('"');
    if s.is_empty() {
        None
    } else {
        Some(PathBuf::from(s))
    }
}

/// Entrée de « Programmes et fonctionnalités ».
#[derive(Debug, Clone)]
pub struct UninstallEntry {
    pub key: String,
    pub name: String,
    pub location: Option<PathBuf>,
    pub icon: Option<PathBuf>,
    pub uninstall: String,
    pub size_kb: Option<u64>,
}

pub fn uninstall_entries() -> Vec<UninstallEntry> {
    const PATH: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
    let mut roots = hklm_views(PATH);
    if let Some(k) = hkcu(PATH) {
        roots.push(k);
    }
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for root in roots {
        for sub in root.enum_keys().flatten() {
            let Ok(k) = root.open_subkey(&sub) else { continue };
            let Some(name) = reg_string(&k, "DisplayName") else { continue };
            if reg_u32(&k, "SystemComponent") == Some(1) {
                continue;
            }
            if !seen.insert(format!("{sub}|{name}")) {
                continue;
            }
            out.push(UninstallEntry {
                key: sub,
                name,
                location: reg_string(&k, "InstallLocation")
                    .map(|s| PathBuf::from(s.trim_matches('"').trim_end_matches('\\'))),
                icon: reg_string(&k, "DisplayIcon").and_then(|s| clean_exe_path(&s)),
                uninstall: reg_string(&k, "UninstallString").unwrap_or_default(),
                size_kb: reg_u32(&k, "EstimatedSize").map(u64::from),
            });
        }
    }
    out
}

// ───────────────────────────── VDF (Valve KeyValues texte) ─────────────────────────────

#[derive(Debug, Clone)]
pub enum Vdf {
    Str(String),
    Obj(Vec<(String, Vdf)>),
}

impl Vdf {
    pub fn get(&self, key: &str) -> Option<&Vdf> {
        match self {
            Vdf::Obj(items) => items
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v),
            Vdf::Str(_) => None,
        }
    }

    pub fn path(&self, keys: &[&str]) -> Option<&Vdf> {
        keys.iter().try_fold(self, |node, k| node.get(k))
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Vdf::Str(s) => Some(s),
            Vdf::Obj(_) => None,
        }
    }

    pub fn str_of(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Vdf::as_str)
    }

    pub fn entries(&self) -> &[(String, Vdf)] {
        match self {
            Vdf::Obj(items) => items,
            Vdf::Str(_) => &[],
        }
    }
}

pub fn parse_vdf(text: &str) -> Option<Vdf> {
    let chars: Vec<char> = text.chars().collect();
    let mut pos = 0usize;
    let items = parse_vdf_object(&chars, &mut pos, 0)?;
    Some(Vdf::Obj(items))
}

fn vdf_skip(chars: &[char], pos: &mut usize) {
    while *pos < chars.len() {
        let c = chars[*pos];
        if c.is_whitespace() {
            *pos += 1;
        } else if c == '/' && chars.get(*pos + 1) == Some(&'/') {
            while *pos < chars.len() && chars[*pos] != '\n' {
                *pos += 1;
            }
        } else {
            break;
        }
    }
}

fn vdf_token(chars: &[char], pos: &mut usize) -> Option<String> {
    vdf_skip(chars, pos);
    let c = *chars.get(*pos)?;
    let mut out = String::new();
    if c == '"' {
        *pos += 1;
        while *pos < chars.len() {
            let c = chars[*pos];
            *pos += 1;
            match c {
                '"' => return Some(out),
                '\\' => {
                    if let Some(&n) = chars.get(*pos) {
                        *pos += 1;
                        out.push(match n {
                            'n' => '\n',
                            't' => '\t',
                            other => other,
                        });
                    }
                }
                other => out.push(other),
            }
        }
        Some(out)
    } else {
        while *pos < chars.len() {
            let c = chars[*pos];
            if c.is_whitespace() || c == '{' || c == '}' || c == '"' {
                break;
            }
            out.push(c);
            *pos += 1;
        }
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }
}

fn parse_vdf_object(chars: &[char], pos: &mut usize, depth: usize) -> Option<Vec<(String, Vdf)>> {
    if depth > 64 {
        return None;
    }
    let mut items = Vec::new();
    loop {
        vdf_skip(chars, pos);
        match chars.get(*pos) {
            None => break,
            Some('}') => {
                *pos += 1;
                break;
            }
            _ => {}
        }
        let Some(key) = vdf_token(chars, pos) else { break };
        vdf_skip(chars, pos);
        match chars.get(*pos) {
            Some('{') => {
                *pos += 1;
                let child = parse_vdf_object(chars, pos, depth + 1)?;
                items.push((key, Vdf::Obj(child)));
            }
            Some(_) => {
                let value = vdf_token(chars, pos).unwrap_or_default();
                items.push((key, Vdf::Str(value)));
            }
            None => break,
        }
    }
    Some(items)
}

// ───────────────────────────── Titres ─────────────────────────────

fn fold_char(c: char) -> char {
    match c {
        'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => 'a',
        'ç' => 'c',
        'è' | 'é' | 'ê' | 'ë' => 'e',
        'ì' | 'í' | 'î' | 'ï' => 'i',
        'ñ' => 'n',
        'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
        'ù' | 'ú' | 'û' | 'ü' => 'u',
        'ý' | 'ÿ' => 'y',
        other => other,
    }
}

/// Minuscule, sans accents ni ponctuation ni ™/®, espaces compactés.
pub fn norm_title(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_space = true;
    for ch in s.chars() {
        if matches!(ch, '™' | '®' | '©' | '\'' | '’' | '`') {
            continue;
        }
        let lowered = ch.to_lowercase().next().unwrap_or(ch);
        let c = fold_char(lowered);
        if c.is_alphanumeric() {
            out.push(c);
            last_space = false;
        } else if !last_space {
            out.push(' ');
            last_space = true;
        }
    }
    out.trim_end().to_string()
}

const EDITION_WORDS: &[&str] = &[
    "edition", "editions", "standard", "deluxe", "ultimate", "gold", "premium", "special",
    "digital", "collector", "collectors", "complete", "goty", "legendary", "ultime", "bundle",
    "launch",
];

const PLATFORM_WORDS: &[&str] = &["pc", "windows", "win10", "eu", "na", "asia", "for", "steam"];

/// Sépare un titre normalisé en (jeu de base, édition éventuelle).
/// « Red Dead Redemption 2: Ultimate Edition » → ("red dead redemption 2", Some("Ultimate Edition")).
pub fn split_edition(title: &str) -> (String, Option<String>) {
    let norm = norm_title(title).replace("game of the year", "goty");
    let mut tokens: Vec<&str> = norm.split(' ').filter(|t| !t.is_empty()).collect();
    let mut stripped: Vec<&str> = Vec::new();
    while tokens.len() > 1 {
        let last = *tokens.last().unwrap();
        if EDITION_WORDS.contains(&last) {
            stripped.push(last);
            tokens.pop();
        } else {
            break;
        }
    }
    stripped.reverse();
    let base = tokens.join(" ");
    let meaningful: Vec<&str> = stripped
        .iter()
        .copied()
        .filter(|t| !matches!(*t, "edition" | "editions" | "standard" | "digital"))
        .collect();
    let edition = if meaningful.is_empty() {
        None
    } else {
        let label = stripped
            .iter()
            .map(|t| match *t {
                "goty" => "GOTY".to_string(),
                other => {
                    let mut c = other.chars();
                    c.next()
                        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                        .unwrap_or_default()
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        Some(label)
    };
    (base, edition)
}

/// Clé de rapprochement entre un jeu installé et un résultat de boutique :
/// base sans édition ni suffixe de plateforme/région (« World of Tanks EU » → « world of tanks »).
pub fn match_key(title: &str) -> String {
    let (base, _) = split_edition(title);
    let mut tokens: Vec<&str> = base.split(' ').collect();
    while tokens.len() > 1 && PLATFORM_WORDS.contains(tokens.last().unwrap()) {
        tokens.pop();
    }
    tokens.join(" ")
}

/// Extrait le texte entre `<tag>` et `</tag>` (premier trouvé).
pub fn xml_tag<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}");
    let start = xml.find(&open)?;
    let after = &xml[start..];
    let gt = after.find('>')?;
    let body = &after[gt + 1..];
    let end = body.find(&format!("</{tag}>"))?;
    Some(body[..end].trim())
}

/// Valeur d'un attribut `name="..."` dans un fragment XML.
pub fn xml_attr<'a>(fragment: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("{name}=\"");
    let mut search = fragment;
    loop {
        let idx = search.find(&needle)?;
        // l'attribut doit commencer sur une frontière (espace ou début)
        let boundary = idx == 0 || search.as_bytes()[idx - 1].is_ascii_whitespace();
        let rest = &search[idx + needle.len()..];
        if boundary {
            let end = rest.find('"')?;
            return Some(&rest[..end]);
        }
        search = rest;
    }
}

pub fn xml_unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vdf_nested() {
        let v = parse_vdf("\"a\"\n{\n\t\"path\"\t\t\"C:\\\\Steam\"\n\t\"apps\"\n\t{\n\t\t\"10\"\t\"1\"\n\t}\n}").unwrap();
        assert_eq!(v.path(&["a", "path"]).and_then(Vdf::as_str), Some("C:\\Steam"));
        assert_eq!(v.path(&["a", "apps", "10"]).and_then(Vdf::as_str), Some("1"));
    }

    #[test]
    fn editions() {
        assert_eq!(
            split_edition("Red Dead Redemption 2: Ultimate Edition"),
            ("red dead redemption 2".into(), Some("Ultimate Edition".into()))
        );
        assert_eq!(split_edition("Forza Horizon 6 Standard Edition").1, None);
        assert_eq!(match_key("World of Tanks EU"), "world of tanks");
        assert_eq!(match_key("EA SPORTS FC™ 26"), match_key("EA SPORTS FC 26"));
        assert_eq!(match_key("Honkai: Star Rail"), match_key("Honkai Star Rail"));
    }

    #[test]
    fn exe_paths() {
        assert_eq!(clean_exe_path("\"D:\\F1 24\\F1_24.exe\"").unwrap(), PathBuf::from("D:\\F1 24\\F1_24.exe"));
        assert_eq!(clean_exe_path("D:\\wot\\game.ico,0").unwrap(), PathBuf::from("D:\\wot\\game.ico"));
    }
}
