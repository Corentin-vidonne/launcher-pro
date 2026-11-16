//! Magasin : recherche d'un jeu dans plusieurs boutiques et comparaison des prix.
//!
//! Chaque boutique renvoie des `RawItem` ; ils sont regroupés par jeu (titre normalisé, éditions
//! rattachées au jeu de base) pour produire une fiche unique avec toutes les offres.

use std::collections::HashMap;

use reqwest::Client;
use serde::Serialize;

use crate::util;

pub mod gog;
pub mod instant_gaming;
pub mod microsoft;
pub mod steam;

/// Identité annoncée aux boutiques : l'application se présente pour ce qu'elle est.
pub const USER_AGENT: &str = concat!("LauncherPro/", env!("CARGO_PKG_VERSION"), " (application de bureau)");

/// Pays et langue de l'utilisateur (prix et textes localisés).
#[derive(Clone, Debug)]
pub struct Region {
    /// Code pays ISO, ex. "FR".
    pub country: String,
    /// Langue courte, ex. "fr".
    pub lang: String,
    /// Locale complète, ex. "fr-FR".
    pub locale: String,
}

impl Region {
    pub fn detect() -> Self {
        let intl = util::hkcu(r"Control Panel\International");
        let locale = intl
            .as_ref()
            .and_then(|k| util::reg_string(k, "LocaleName"))
            .filter(|l| l.len() >= 2)
            .unwrap_or_else(|| "fr-FR".into());
        let country = util::hkcu(r"Control Panel\International\Geo")
            .and_then(|k| util::reg_string(&k, "Name"))
            .filter(|c| c.len() == 2)
            .or_else(|| locale.split('-').nth(1).map(str::to_string))
            .unwrap_or_else(|| "FR".into())
            .to_uppercase();
        let lang = locale.split('-').next().unwrap_or("fr").to_lowercase();
        Region { country, lang, locale }
    }

    /// Nom de langue attendu par les API Steam.
    pub fn steam_language(&self) -> &'static str {
        match self.lang.as_str() {
            "fr" => "french",
            "de" => "german",
            "es" => "spanish",
            "it" => "italian",
            "pt" => "portuguese",
            "nl" => "dutch",
            "pl" => "polish",
            _ => "english",
        }
    }
}

/// Résultat brut d'une boutique.
#[derive(Clone, Debug, Default)]
pub struct RawItem {
    pub store: &'static str,
    pub title: String,
    pub price: Option<f64>,
    pub regular: Option<f64>,
    pub currency: String,
    pub free: bool,
    pub url: String,
    /// Visuel paysage (bannière).
    pub image: Option<String>,
    /// Jaquette portrait.
    pub cover: Option<String>,
    pub hero: Option<String>,
    pub description: Option<String>,
    pub note: Option<String>,
    pub is_dlc: bool,
    pub steam_appid: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Offer {
    pub store: String,
    pub store_name: String,
    pub title: String,
    pub edition: Option<String>,
    pub price: Option<f64>,
    pub regular: Option<f64>,
    pub discount: Option<u32>,
    pub currency: String,
    pub free: bool,
    pub url: String,
    pub note: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreHit {
    pub key: String,
    pub title: String,
    pub image: Option<String>,
    pub cover: Option<String>,
    pub hero: Option<String>,
    pub description: Option<String>,
    pub steam_appid: Option<u32>,
    pub offers: Vec<Offer>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub hits: Vec<StoreHit>,
    /// Boutiques qui n'ont pas répondu (affiché discrètement dans l'interface).
    pub failed: Vec<String>,
}

pub fn store_name(store: &str) -> &'static str {
    match store {
        "steam" => "Steam",
        "epic" => "Epic Games",
        "microsoft" => "Microsoft Store",
        "gog" => "GOG",
        "instantgaming" => "Instant Gaming",
        _ => "Boutique",
    }
}

/// Source (launcher) correspondant à une boutique ; `None` = revendeur de clés, toujours affiché.
pub fn store_source(store: &str) -> Option<&'static str> {
    match store {
        "steam" => Some("steam"),
        "epic" => Some("epic"),
        "microsoft" => Some("xbox"),
        "gog" => Some("gog"),
        _ => None,
    }
}

/// Recherche dans toutes les boutiques pertinentes, en parallèle.
/// `enabled` indique si une boutique doit apparaître dans les offres.
pub async fn search(
    http: &Client,
    region: &Region,
    query: &str,
    enabled: impl Fn(&str) -> bool,
) -> SearchResult {
    let want = |store: &str| enabled(store);
    // Steam est toujours interrogé : il sert de référentiel (titres, visuels, fiche détaillée).
    let (steam, microsoft, gog, ig) = tokio::join!(
        steam::search(http, region, query),
        maybe(want("microsoft"), microsoft::search(http, region, query)),
        maybe(want("gog"), gog::search(http, region, query)),
        maybe(want("instantgaming"), instant_gaming::search(http, region, query)),
    );

    let mut failed = Vec::new();
    let mut items: Vec<RawItem> = Vec::new();
    for (store, result) in [
        ("steam", steam),
        ("microsoft", microsoft),
        ("gog", gog),
        ("instantgaming", ig),
    ] {
        match result {
            Ok(list) => items.extend(list),
            Err(_) => failed.push(store_name(store).to_string()),
        }
    }
    SearchResult { hits: group(items, query, &enabled), failed }
}

async fn maybe<F>(run: bool, fut: F) -> Result<Vec<RawItem>, String>
where
    F: std::future::Future<Output = Result<Vec<RawItem>, String>>,
{
    if run {
        fut.await
    } else {
        Ok(Vec::new())
    }
}

fn group(items: Vec<RawItem>, query: &str, enabled: &impl Fn(&str) -> bool) -> Vec<StoreHit> {
    struct Group {
        hit: StoreHit,
        has_base_title: bool,
        order: usize,
    }
    let mut groups: Vec<Group> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();

    for item in items {
        let key = util::match_key(&item.title);
        if key.is_empty() {
            continue;
        }
        let (_, edition) = util::split_edition(&item.title);
        let slot = match index.get(&key) {
            Some(i) => *i,
            None => {
                // Un contenu additionnel ne crée pas de fiche à lui seul.
                if item.is_dlc {
                    continue;
                }
                let order = groups.len();
                groups.push(Group {
                    hit: StoreHit {
                        key: key.clone(),
                        title: item.title.clone(),
                        image: None,
                        cover: None,
                        hero: None,
                        description: None,
                        steam_appid: None,
                        offers: Vec::new(),
                    },
                    has_base_title: edition.is_none(),
                    order,
                });
                index.insert(key, order);
                order
            }
        };
        let g = &mut groups[slot];
        // Titre de la fiche = celui de l'édition de base, de préférence.
        if !g.has_base_title && edition.is_none() && !item.is_dlc {
            g.hit.title = item.title.clone();
            g.has_base_title = true;
        }
        if edition.is_none() || g.hit.image.is_none() {
            g.hit.image = g.hit.image.take().or(item.image.clone());
        }
        g.hit.cover = g.hit.cover.take().or(item.cover.clone());
        g.hit.hero = g.hit.hero.take().or(item.hero.clone());
        g.hit.description = g.hit.description.take().or(item.description.clone());
        if edition.is_none() {
            g.hit.steam_appid = g.hit.steam_appid.or(item.steam_appid);
        }

        if !enabled(item.store) || (item.price.is_none() && !item.free) {
            continue;
        }
        let discount = match (item.price, item.regular) {
            (Some(p), Some(r)) if r > p && r > 0.0 => Some((((r - p) / r) * 100.0).round() as u32),
            _ => None,
        };
        g.hit.offers.push(Offer {
            store: item.store.to_string(),
            store_name: store_name(item.store).to_string(),
            title: item.title,
            edition,
            price: item.price,
            regular: item.regular.filter(|r| item.price.map(|p| *r > p).unwrap_or(false)),
            discount,
            currency: item.currency,
            free: item.free,
            url: item.url,
            note: item.note,
        });
    }

    // Pertinence : correspondance exacte, puis préfixe, puis tous les mots présents.
    let q = util::match_key(query);
    let q_tokens: Vec<&str> = q.split(' ').filter(|t| !t.is_empty()).collect();
    let rank = |key: &str| -> u8 {
        if key == q {
            0
        } else if key.starts_with(&q) {
            1
        } else if q_tokens.iter().all(|t| key.split(' ').any(|k| k == *t)) {
            2
        } else {
            3
        }
    };
    groups.sort_by_key(|g| (rank(&g.hit.key), g.order));

    groups
        .into_iter()
        // Les suggestions approximatives ne sont gardées que si elles viennent du référentiel Steam
        // (les autres boutiques remontent beaucoup de résultats sans rapport).
        .filter(|g| rank(&g.hit.key) < 3 || g.hit.steam_appid.is_some())
        .map(|mut g| {
            // Édition de base d'abord, puis du moins cher au plus cher.
            g.hit.offers.sort_by(|a, b| {
                let price = |o: &Offer| if o.free { 0.0 } else { o.price.unwrap_or(f64::MAX) };
                (a.edition.is_some(), price(a))
                    .partial_cmp(&(b.edition.is_some(), price(b)))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            // Une même boutique liste parfois deux fois le même produit : on garde le moins cher
            // (le tri ci-dessus garantit qu'il arrive en premier).
            let mut seen = std::collections::HashSet::new();
            g.hit
                .offers
                .retain(|o| seen.insert((o.store.clone(), util::norm_title(&o.title), o.note.clone())));
            g.hit
        })
        .take(30)
        .collect()
}

// ───────────────────────────── Aides communes aux boutiques ─────────────────────────────

pub(crate) fn http_err(e: reqwest::Error) -> String {
    e.to_string()
}

/// Nombre JSON ou chaîne numérique (« 17.39 ») → f64.
pub(crate) fn as_price(v: &serde_json::Value) -> Option<f64> {
    match v {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.trim().replace(',', ".").parse::<f64>().ok(),
        _ => None,
    }
}

/// HTML → texte brut lisible (paragraphes conservés). Jamais d'HTML distant injecté dans l'interface.
pub(crate) fn html_to_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut chars = html.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' {
            let mut tag = String::new();
            for t in chars.by_ref() {
                if t == '>' {
                    break;
                }
                tag.push(t);
            }
            let name = tag.trim_start_matches('/').split_whitespace().next().unwrap_or("").to_lowercase();
            match name.as_str() {
                "br" | "br/" | "p" | "div" | "h1" | "h2" | "h3" | "ul" | "ol" => out.push('\n'),
                "li" if !tag.starts_with('/') => out.push_str("\n• "),
                _ => {}
            }
        } else {
            out.push(c);
        }
    }
    let decoded = out
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">");
    let mut lines: Vec<String> = Vec::new();
    for line in decoded.lines().map(|l| l.split_whitespace().collect::<Vec<_>>().join(" ")) {
        if line.is_empty() && lines.last().map(|l| l.is_empty()).unwrap_or(true) {
            continue;
        }
        lines.push(line);
    }
    lines.join("\n").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(store: &'static str, title: &str, price: f64) -> RawItem {
        RawItem { store, title: title.into(), price: Some(price), currency: "EUR".into(), ..Default::default() }
    }

    #[test]
    fn groups_editions_under_base_game() {
        let items = vec![
            item("steam", "Red Dead Redemption 2", 59.99),
            item("epic", "Red Dead Redemption 2", 59.99),
            item("instantgaming", "Red Dead Redemption 2: Ultimate Edition", 17.39),
            item("instantgaming", "Red Dead Redemption 2", 12.99),
            RawItem { steam_appid: Some(4147110), ..item("steam", "Deadrock Redemption 2", 14.79) },
        ];
        let hits = group(items, "red dead redemption 2", &|_| true);
        assert_eq!(hits[0].title, "Red Dead Redemption 2");
        assert_eq!(hits[0].offers.len(), 4);
        // Le moins cher de l'édition de base en premier, l'édition Ultimate en dernier.
        assert_eq!(hits[0].offers[0].store, "instantgaming");
        assert_eq!(hits[0].offers[0].price, Some(12.99));
        assert_eq!(hits[0].offers[3].edition.as_deref(), Some("Ultimate Edition"));
        assert_eq!(hits[1].title, "Deadrock Redemption 2");
    }

    #[test]
    fn dedupes_and_drops_noise() {
        let mut steam = item("steam", "Forza Horizon 5", 59.99);
        steam.steam_appid = Some(1551360);
        let mut dlc = item("steam", "Forza Horizon 5 Car Pass", 29.99);
        dlc.is_dlc = true;
        let items = vec![
            steam,
            dlc,
            item("instantgaming", "Forza Horizon 5", 27.21),
            item("instantgaming", "Forza Horizon 5", 26.09),
            item("microsoft", "Call of Duty - Amélioration", 30.0),
        ];
        let hits = group(items, "forza horizon", &|_| true);
        assert_eq!(hits.len(), 1, "ni DLC ni résultat hors sujet");
        let ig: Vec<_> = hits[0].offers.iter().filter(|o| o.store == "instantgaming").collect();
        assert_eq!(ig.len(), 1);
        assert_eq!(ig[0].price, Some(26.09));
    }

    #[test]
    fn hides_disabled_stores_but_keeps_metadata() {
        let mut steam = item("steam", "Cyberpunk 2077", 59.99);
        steam.steam_appid = Some(1091500);
        let hits = group(vec![steam, item("gog", "Cyberpunk 2077", 29.99)], "cyberpunk", &|s| s != "gog");
        assert_eq!(hits[0].steam_appid, Some(1091500));
        assert_eq!(hits[0].offers.len(), 1);
    }

    /// Requêtes réelles vers les boutiques :
    /// `cargo test live_search -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn live_search() {
        let http = Client::builder().user_agent(USER_AGENT).gzip(true).build().unwrap();
        let region = Region::detect();
        println!("région : {region:?}");
        for query in ["red dead redemption 2", "forza horizon", "cyberpunk"] {
            let result = search(&http, &region, query, |_| true).await;
            println!("\n=== {query} — échecs : {:?}", result.failed);
            for hit in result.hits.iter().take(4) {
                println!("• {} (steam={:?}) image={} cover={}", hit.title, hit.steam_appid, hit.image.is_some(), hit.cover.is_some());
                for o in &hit.offers {
                    println!(
                        "    {:<15} {:<52} {:>8} {} (-{}%) {:?} {}",
                        o.store_name,
                        o.title,
                        o.price.map(|p| format!("{p:.2}")).unwrap_or_else(|| "gratuit".into()),
                        o.currency,
                        o.discount.unwrap_or(0),
                        o.note,
                        o.url
                    );
                }
            }
            assert!(!result.hits.is_empty(), "aucun résultat pour {query}");
        }
        let details = steam::details(&http, &region, 1174180).await.unwrap();
        println!("\nfiche : {} — {:?} — {} captures — {:?}", details.name, details.genres, details.screenshots.len(), details.release_date);
        let featured = steam::featured(&http, &region).await.unwrap();
        println!("à la une : {} promos, {} meilleures ventes, {} nouveautés", featured.specials.len(), featured.top_sellers.len(), featured.new_releases.len());
    }

    #[test]
    fn strips_html() {
        assert_eq!(html_to_text("<h1>Titre</h1><p>Un &amp; deux<br>trois</p>"), "Titre\n\nUn & deux\ntrois");
    }
}
