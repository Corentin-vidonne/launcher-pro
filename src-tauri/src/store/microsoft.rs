//! Microsoft Store / Xbox PC : catalogue public `displaycatalog`.

use reqwest::Client;
use serde_json::Value;

use super::{http_err, RawItem, Region};

pub async fn search(http: &Client, region: &Region, query: &str) -> Result<Vec<RawItem>, String> {
    let body: Value = http
        .get("https://displaycatalog.mp.microsoft.com/v7.0/productFamilies/Games/products")
        .query(&[
            ("query", query),
            ("market", region.country.as_str()),
            ("languages", region.locale.as_str()),
            ("$top", "10"),
            ("fieldsTemplate", "details"),
        ])
        .send()
        .await
        .map_err(http_err)?
        .error_for_status()
        .map_err(http_err)?
        .json()
        .await
        .map_err(http_err)?;

    let products = body["Products"].as_array().cloned().unwrap_or_default();
    Ok(products.iter().filter_map(|p| parse(p, region)).collect())
}

fn parse(p: &Value, region: &Region) -> Option<RawItem> {
    // « Durable » = contenu additionnel ; on ne garde que les jeux.
    if p["ProductKind"].as_str() != Some("Game") {
        return None;
    }
    let id = p["ProductId"].as_str()?;
    let lp = p["LocalizedProperties"].as_array()?.first()?;
    let title = lp["ProductTitle"].as_str()?.to_string();

    // Prix : première disponibilité achetable sur PC, sans condition d'abonnement.
    let mut price: Option<(f64, f64, String)> = None;
    'skus: for sku in p["DisplaySkuAvailabilities"].as_array()? {
        for av in sku["Availabilities"].as_array().into_iter().flatten() {
            let actions = av["Actions"].as_array();
            let purchasable = actions
                .map(|a| a.iter().any(|x| x == "Purchase"))
                .unwrap_or(false);
            let on_pc = av["Conditions"]["ClientConditions"]["AllowedPlatforms"]
                .as_array()
                .map(|pl| pl.iter().any(|x| x["PlatformName"] == "Windows.Desktop"))
                .unwrap_or(false);
            let member_only = !av["AffirmationId"].is_null() || av["RemediationRequired"].as_bool() == Some(true);
            if !purchasable || !on_pc || member_only {
                continue;
            }
            let pr = &av["OrderManagementData"]["Price"];
            if let Some(list) = pr["ListPrice"].as_f64() {
                price = Some((
                    list,
                    pr["MSRP"].as_f64().unwrap_or(list),
                    pr["CurrencyCode"].as_str().unwrap_or("EUR").to_string(),
                ));
                break 'skus;
            }
        }
    }
    let (list, msrp, currency) = price?; // pas vendu sur PC → ignoré

    let image = |purpose: &str, size: &str| {
        lp["Images"]
            .as_array()?
            .iter()
            .find(|i| i["ImagePurpose"] == purpose)
            .and_then(|i| i["Uri"].as_str())
            .map(|u| {
                let base = if u.starts_with("//") { format!("https:{u}") } else { u.to_string() };
                format!("{base}?{size}")
            })
    };
    let hero = image("SuperHeroArt", "w=1920").or_else(|| image("TitledHeroArt", "w=1920"));
    Some(RawItem {
        store: "microsoft",
        title,
        price: Some(list).filter(|p| *p > 0.0),
        regular: Some(msrp),
        currency,
        free: list == 0.0,
        url: format!("https://apps.microsoft.com/detail/{id}?hl={}&gl={}", region.locale, region.country),
        image: image("SuperHeroArt", "w=920").or_else(|| image("BoxArt", "w=600")),
        cover: image("Poster", "w=600&h=900"),
        hero,
        description: lp["ShortDescription"]
            .as_str()
            .filter(|d| d.len() > 20)
            .map(str::to_string),
        ..Default::default()
    })
}
