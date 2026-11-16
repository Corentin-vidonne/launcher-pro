//! Steam : recherche (storesearch), visuels (IStoreBrowseService/GetItems), fiche (appdetails),
//! sélection de la page d'accueil (featuredcategories).

use std::collections::HashMap;

use reqwest::Client;
use serde::Serialize;
use serde_json::{json, Value};

use super::{html_to_text, http_err, RawItem, Region};

const ASSET_BASE: &str = "https://shared.steamstatic.com/store_item_assets/";

#[derive(Clone, Debug, Default)]
pub struct Assets {
    pub header: Option<String>,
    pub cover: Option<String>,
    pub hero: Option<String>,
    pub is_free: bool,
    /// Démo, DLC ou bande-son : rattaché à un jeu, jamais une fiche à part entière.
    pub is_extra: bool,
}

/// Visuels officiels d'une liste d'applications (une seule requête).
pub async fn assets(http: &Client, region: &Region, appids: &[u32]) -> HashMap<u32, Assets> {
    let mut out = HashMap::new();
    if appids.is_empty() {
        return out;
    }
    let input = json!({
        "ids": appids.iter().map(|id| json!({ "appid": id })).collect::<Vec<_>>(),
        "context": { "language": region.steam_language(), "country_code": region.country },
        "data_request": { "include_assets": true }
    });
    let Ok(resp) = http
        .get("https://api.steampowered.com/IStoreBrowseService/GetItems/v1")
        .query(&[("input_json", input.to_string())])
        .send()
        .await
    else {
        return out;
    };
    let Ok(body) = resp.json::<Value>().await else { return out };
    for item in body["response"]["store_items"].as_array().into_iter().flatten() {
        let Some(appid) = item["appid"].as_u64() else { continue };
        let a = &item["assets"];
        let format = a["asset_url_format"].as_str().unwrap_or("");
        let url = |name: &str| {
            a[name]
                .as_str()
                .filter(|f| !f.is_empty() && !format.is_empty())
                .map(|f| format!("{ASSET_BASE}{}", format.replace("${FILENAME}", f)))
        };
        out.insert(
            appid as u32,
            Assets {
                header: url("header"),
                cover: url("library_capsule"),
                hero: url("library_hero"),
                is_free: item["is_free"].as_bool().unwrap_or(false),
                // Type d'article Steam : 0 jeu, 1 démo, 4 DLC, 11 musique.
                is_extra: matches!(item["type"].as_u64(), Some(1 | 4 | 11)),
            },
        );
    }
    out
}

pub async fn search(http: &Client, region: &Region, query: &str) -> Result<Vec<RawItem>, String> {
    let body: Value = http
        .get("https://store.steampowered.com/api/storesearch/")
        .query(&[("term", query), ("l", region.steam_language()), ("cc", region.country.as_str())])
        .send()
        .await
        .map_err(http_err)?
        .error_for_status()
        .map_err(http_err)?
        .json()
        .await
        .map_err(http_err)?;

    let apps: Vec<&Value> = body["items"]
        .as_array()
        .map(|a| a.iter().filter(|i| i["type"] == "app").collect())
        .unwrap_or_default();
    let ids: Vec<u32> = apps.iter().filter_map(|i| i["id"].as_u64()).map(|n| n as u32).collect();
    let visuals = assets(http, region, &ids).await;

    Ok(apps
        .into_iter()
        .filter_map(|i| {
            let appid = i["id"].as_u64()? as u32;
            let art = visuals.get(&appid).cloned().unwrap_or_default();
            let cents = |v: &Value| v.as_f64().map(|c| c / 100.0);
            let price = cents(&i["price"]["final"]);
            Some(RawItem {
                store: "steam",
                title: i["name"].as_str()?.to_string(),
                price,
                regular: cents(&i["price"]["initial"]),
                currency: i["price"]["currency"].as_str().unwrap_or("EUR").to_string(),
                free: price.is_none() && art.is_free,
                url: format!("https://store.steampowered.com/app/{appid}/"),
                image: art.header.or_else(|| i["tiny_image"].as_str().map(str::to_string)),
                cover: art.cover,
                hero: art.hero,
                is_dlc: art.is_extra,
                steam_appid: Some(appid),
                ..Default::default()
            })
        })
        .collect())
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Screenshot {
    pub thumb: String,
    pub full: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Details {
    pub appid: u32,
    pub name: String,
    pub summary: String,
    pub about: String,
    pub header: Option<String>,
    pub screenshots: Vec<Screenshot>,
    pub genres: Vec<String>,
    pub features: Vec<String>,
    pub developers: Vec<String>,
    pub publishers: Vec<String>,
    pub release_date: Option<String>,
    pub coming_soon: bool,
    pub metacritic: Option<u64>,
    pub recommendations: Option<u64>,
    pub website: Option<String>,
    pub requirements: Option<String>,
}

pub async fn details(http: &Client, region: &Region, appid: u32) -> Result<Details, String> {
    let body: Value = http
        .get("https://store.steampowered.com/api/appdetails")
        .query(&[
            ("appids", appid.to_string()),
            ("cc", region.country.clone()),
            ("l", region.steam_language().to_string()),
        ])
        .send()
        .await
        .map_err(http_err)?
        .error_for_status()
        .map_err(http_err)?
        .json()
        .await
        .map_err(http_err)?;
    let entry = &body[appid.to_string()];
    if entry["success"].as_bool() != Some(true) {
        return Err("Fiche indisponible sur Steam.".into());
    }
    let d = &entry["data"];
    let strings = |v: &Value| -> Vec<String> {
        v.as_array()
            .map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect())
            .unwrap_or_default()
    };
    let labels = |v: &Value| -> Vec<String> {
        v.as_array()
            .map(|a| a.iter().filter_map(|g| g["description"].as_str().map(str::to_string)).collect())
            .unwrap_or_default()
    };
    let text = |v: &Value| v.as_str().map(html_to_text).filter(|s| !s.is_empty());

    Ok(Details {
        appid,
        name: d["name"].as_str().unwrap_or("").to_string(),
        summary: text(&d["short_description"]).unwrap_or_default(),
        about: text(&d["about_the_game"]).unwrap_or_default(),
        header: d["header_image"].as_str().map(str::to_string),
        screenshots: d["screenshots"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|s| {
                        Some(Screenshot {
                            thumb: s["path_thumbnail"].as_str()?.to_string(),
                            full: s["path_full"].as_str()?.to_string(),
                        })
                    })
                    .take(12)
                    .collect()
            })
            .unwrap_or_default(),
        genres: labels(&d["genres"]),
        features: labels(&d["categories"]).into_iter().take(8).collect(),
        developers: strings(&d["developers"]),
        publishers: strings(&d["publishers"]),
        release_date: d["release_date"]["date"].as_str().filter(|s| !s.is_empty()).map(str::to_string),
        coming_soon: d["release_date"]["coming_soon"].as_bool().unwrap_or(false),
        metacritic: d["metacritic"]["score"].as_u64(),
        recommendations: d["recommendations"]["total"].as_u64(),
        website: d["website"].as_str().filter(|s| s.starts_with("http")).map(str::to_string),
        requirements: text(&d["pc_requirements"]["minimum"]),
    })
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeaturedItem {
    pub appid: u32,
    pub title: String,
    pub image: Option<String>,
    pub price: Option<f64>,
    pub regular: Option<f64>,
    pub discount: u32,
    pub currency: String,
}

#[derive(Clone, Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Featured {
    pub specials: Vec<FeaturedItem>,
    pub top_sellers: Vec<FeaturedItem>,
    pub new_releases: Vec<FeaturedItem>,
}

pub async fn featured(http: &Client, region: &Region) -> Result<Featured, String> {
    let body: Value = http
        .get("https://store.steampowered.com/api/featuredcategories")
        .query(&[("cc", region.country.as_str()), ("l", region.steam_language())])
        .send()
        .await
        .map_err(http_err)?
        .error_for_status()
        .map_err(http_err)?
        .json()
        .await
        .map_err(http_err)?;

    let list = |key: &str| -> Vec<FeaturedItem> {
        let mut seen = std::collections::HashSet::new();
        body[key]["items"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|i| i["type"].as_u64().unwrap_or(0) == 0)
            .filter_map(|i| {
                let appid = i["id"].as_u64()? as u32;
                if !seen.insert(appid) {
                    return None;
                }
                let cents = |v: &Value| v.as_f64().map(|c| c / 100.0);
                Some(FeaturedItem {
                    appid,
                    title: i["name"].as_str()?.to_string(),
                    image: i["header_image"]
                        .as_str()
                        .or(i["large_capsule_image"].as_str())
                        .map(str::to_string),
                    price: cents(&i["final_price"]),
                    regular: cents(&i["original_price"]),
                    discount: i["discount_percent"].as_u64().unwrap_or(0) as u32,
                    currency: i["currency"].as_str().unwrap_or("EUR").to_string(),
                })
            })
            .take(12)
            .collect()
    };
    Ok(Featured {
        specials: list("specials"),
        top_sellers: list("top_sellers"),
        new_releases: list("new_releases"),
    })
}
