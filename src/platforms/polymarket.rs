use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PolymarketError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("Failed to parse response: {0}")]
    ParseError(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Trade {
    pub id: String,
    pub market: String,
    pub asset_id: String,
    pub side: String,
    pub size: f64,
    pub price: f64,
    pub timestamp: String,
    #[serde(skip)]
    pub market_title: Option<String>,
    #[serde(skip)]
    pub outcome: Option<String>,
    #[serde(skip)]
    pub wallet_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TradesResponse {
    #[serde(default)]
    data: Vec<ActivityItem>,
}

#[derive(Debug, Deserialize)]
struct ActivityItem {
    #[serde(rename = "transactionHash")]
    id: String,
    #[serde(rename = "conditionId")]
    market: Option<String>,
    #[serde(rename = "asset")]
    asset: Option<String>,
    side: Option<String>,
    size: Option<f64>,
    price: Option<f64>,
    timestamp: Option<i64>,
    #[serde(rename = "name")]
    user: Option<String>,
    maker: Option<String>,
    #[serde(rename = "proxyWallet")]
    proxy_wallet: Option<String>,
    // New API includes these fields directly
    title: Option<String>,
    outcome: Option<String>,
}

pub async fn fetch_market_context(condition_id: &str) -> Option<crate::alerts::MarketContext> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://gamma-api.polymarket.com/markets?condition_ids={}",
        condition_id
    );

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .ok()?;

    if !response.status().is_success() {
        return None;
    }

    let text = response.text().await.ok()?;
    let markets: Vec<serde_json::Value> = serde_json::from_str(&text).ok()?;
    let market = markets.first()?;

    let yes_price = market.get("outcomePrices")
        .and_then(|v| v.as_str())
        .and_then(|s| {
            // outcomePrices is a JSON string like "[\"0.65\",\"0.35\"]"
            let prices: Vec<String> = serde_json::from_str(s).ok()?;
            prices.first()?.parse::<f64>().ok()
        })
        .or_else(|| {
            market.get("bestBid").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok())
        })
        .unwrap_or(0.0);

    let no_price = 1.0 - yes_price;

    let spread = market.get("spread")
        .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0.0);

    let volume_24h = market.get("volume24hr")
        .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0.0);

    let open_interest = market.get("openInterest")
        .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0.0);

    let price_change_24h = market.get("oneDayPriceChange")
        .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0.0);

    let liquidity = market.get("liquidityClob")
        .or_else(|| market.get("liquidity"))
        .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0.0);

    Some(crate::alerts::MarketContext {
        yes_price,
        no_price,
        spread,
        volume_24h,
        open_interest,
        price_change_24h,
        liquidity,
    })
}

pub async fn fetch_recent_trades() -> Result<Vec<Trade>, PolymarketError> {
    let client = reqwest::Client::new();

    // Use the Polymarket Data API to fetch recent activity
    // This is a public endpoint that doesn't require authentication
    let url = "https://data-api.polymarket.com/trades";

    let response = client
        .get(url)
        .query(&[("limit", "100")])
        .header("Accept", "application/json")
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(PolymarketError::ParseError(format!(
            "API returned status: {}",
            response.status()
        )));
    }

    let text = response.text().await?;

    // Try to parse as array first (some endpoints return arrays directly)
    if let Ok(items) = serde_json::from_str::<Vec<ActivityItem>>(&text) {
        let trades = items
            .into_iter()
            .filter_map(|item| {
                // Skip trades missing critical data
                let market = item.market?;
                let asset_id = item.asset?;
                let side = item.side?;
                let size = item.size?;
                let price = item.price?;

                Some(Trade {
                    id: item.id.clone(),
                    market,
                    asset_id,
                    side,
                    size,
                    price,
                    timestamp: item
                        .timestamp
                        .and_then(|ts| {
                            chrono::DateTime::from_timestamp(ts, 0)
                                .map(|dt| dt.to_rfc3339())
                        })
                        .unwrap_or_else(|| format!("timestamp_error_{}", item.id)),
                    // New API includes title and outcome directly
                    market_title: item.title,
                    outcome: item.outcome,
                    wallet_id: item.proxy_wallet.or(item.user).or(item.maker),
                })
            })
            .collect();
        return Ok(trades);
    }

    // Try wrapped response format
    if let Ok(wrapped) = serde_json::from_str::<TradesResponse>(&text) {
        let trades = wrapped
            .data
            .into_iter()
            .filter_map(|item| {
                // Skip trades missing critical data
                let market = item.market?;
                let asset_id = item.asset?;
                let side = item.side?;
                let size = item.size?;
                let price = item.price?;

                Some(Trade {
                    id: item.id.clone(),
                    market,
                    asset_id,
                    side,
                    size,
                    price,
                    timestamp: item
                        .timestamp
                        .and_then(|ts| {
                            chrono::DateTime::from_timestamp(ts, 0)
                                .map(|dt| dt.to_rfc3339())
                        })
                        .unwrap_or_else(|| format!("timestamp_error_{}", item.id)),
                    // New API includes title and outcome directly
                    market_title: item.title,
                    outcome: item.outcome,
                    wallet_id: item.proxy_wallet.or(item.user).or(item.maker),
                })
            })
            .collect();
        return Ok(trades);
    }

    // If parsing fails, return empty list rather than error
    // This allows the tool to continue working even if Polymarket API format changes
    Ok(Vec::new())
}
