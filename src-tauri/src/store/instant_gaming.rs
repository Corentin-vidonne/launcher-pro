//! Instant Gaming : la page de recherche publique embarque ses résultats en JSON
//! (`window.searchResults = {...}`) ; on charge cette page, comme un navigateur, et on lit ce bloc.

use reqwest::Client;
use serde_json::Value;

use super::{as_price, http_err, RawItem, Region};

const MARKER: &str = "window.searchResults = ";

pub async fn search(http: &Client, region: &Region, query: &str) -> Result<Vec<RawItem>, String> {
    let (lang, path) = if region.lang == "fr" { ("fr", "rechercher") } else { ("en", "search") };
    let html = http
        .get(format!("https://www.instant-gaming.com/{lang}/{path}/"))
        .query(&[("q", query)])
        .send()
        .await
        .map_err(http_err)?
        .error_for_status()
        .map_err(http_err)?
        .text()
        .await
        .map_err(http_err)?;

    let start = html.find(MARKER).ok_or_else(|| "Résultats Instant Gaming introuvables".to_string())?;
    // Le flux JSON s'arrête tout seul à la fin de l'objet, le reste de la page est ignoré.
    let data: Value = serde_json::Deserializer::from_str(&html[start + MARKER.len()..])
        .into_iter::<Value>()
        .next()
        .and_then(Result::ok)
        .ok_or_else(|| "Résultats Instant Gaming illisibles".to_string())?;

    Ok(data["hits"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|h| parse(h, lang))
        .collect())
}

fn parse(h: &Value, lang: &str) -> Option<RawItem> {
    let on_pc = h["platform_names"]
        .as_array()
        .map(|p| p.iter().any(|x| x == "PC"))
        .unwrap_or(false);
    let in_stock = h["has_stock"].as_i64().unwrap_or(1) != 0;
    let flag = |k: &str| h[k].as_i64().unwrap_or(0) != 0;
    if !on_pc || !in_stock || flag("is_prepaid") || flag("is_subscription") || flag("is_draft") {
        return None;
    }
    let id = h["prod_id"].as_i64()?;
    let seo = h["seo_name"].as_str()?;
    let price = as_price(&h["price"]).filter(|p| *p > 0.0)?;
    // Plateforme d'activation de la clé (Steam, Rockstar, EA App…).
    let platform = h["type"].as_str().filter(|t| !t.is_empty());
    Some(RawItem {
        store: "instantgaming",
        title: h["name"].as_str()?.to_string(),
        price: Some(price),
        regular: as_price(&h["retail"]).filter(|r| *r > price),
        currency: "EUR".into(),
        free: false,
        url: format!("https://www.instant-gaming.com/{lang}/{id}-acheter-{seo}/"),
        image: Some(format!(
            "https://gaming-cdn.com/images/products/{id}/616x353/{seo}-cover.jpg?v={}",
            h["updated_at"].as_i64().unwrap_or(0)
        )),
        note: platform.map(|p| format!("Clé {p}")),
        is_dlc: flag("is_dlc"),
        ..Default::default()
    })
}
