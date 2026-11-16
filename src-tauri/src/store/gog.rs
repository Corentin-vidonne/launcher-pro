//! GOG : API catalogue publique.

use reqwest::Client;
use serde_json::Value;

use super::{as_price, http_err, RawItem, Region};

pub async fn search(http: &Client, region: &Region, query: &str) -> Result<Vec<RawItem>, String> {
    let body: Value = http
        .get("https://catalog.gog.com/v1/catalog")
        .query(&[
            ("limit", "10"),
            ("query", format!("like:{query}").as_str()),
            ("order", "desc:score"),
            ("productType", "in:game,pack"),
            ("countryCode", region.country.as_str()),
            ("locale", region.locale.as_str()),
            ("currencyCode", "EUR"),
        ])
        .send()
        .await
        .map_err(http_err)?
        .error_for_status()
        .map_err(http_err)?
        .json()
        .await
        .map_err(http_err)?;

    Ok(body["products"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| {
            let title = p["title"].as_str()?.to_string();
            let price = as_price(&p["price"]["finalMoney"]["amount"]);
            let https = |v: &Value| v.as_str().filter(|u| u.starts_with("https://")).map(str::to_string);
            Some(RawItem {
                store: "gog",
                title,
                price: price.filter(|p| *p > 0.0),
                regular: as_price(&p["price"]["baseMoney"]["amount"]),
                currency: p["price"]["finalMoney"]["currency"].as_str().unwrap_or("EUR").to_string(),
                free: price == Some(0.0),
                url: https(&p["storeLink"])?,
                image: https(&p["coverHorizontal"]),
                cover: https(&p["coverVertical"]),
                ..Default::default()
            })
        })
        .collect())
}
