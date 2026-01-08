use crate::config::Config;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KalshiError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("Failed to parse response: {0}")]
    ParseError(String),
    #[error("Authentication failed: {0}")]
    AuthError(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Trade {
    #[serde(rename = "trade_id")]
    pub trade_id: String,
    #[serde(rename = "ticker")]
    pub ticker: String,
    #[serde(rename = "price")]
    pub price: f64,
    #[serde(rename = "count")]
    pub count: i32,
    #[serde(rename = "yes_price")]
    pub yes_price: f64,
    #[serde(rename = "no_price")]
    pub no_price: f64,
    #[serde(rename = "taker_side")]
    pub taker_side: String,
    #[serde(rename = "created_time")]
    pub created_time: String,
    #[serde(skip)]
    pub market_title: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TradesResponse {
    #[serde(default)]
    trades: Vec<Trade>,
    #[serde(default)]
    cursor: Option<String>,
}

pub async fn fetch_recent_trades(config: Option<&Config>) -> Result<Vec<Trade>, KalshiError> {
    let client = reqwest::Client::new();
    
    // Kalshi's public trades endpoint
    let url = "https://api.elections.kalshi.com/trade-api/v2/markets/trades";
    
    let mut request = client
        .get(url)
        .query(&[("limit", "100")])
        .header("Accept", "application/json");

    // Add authentication if credentials are provided
    if let Some(cfg) = config {
        if let (Some(key_id), Some(_private_key)) = (&cfg.kalshi_api_key_id, &cfg.kalshi_private_key) {
            // For simplicity, we'll use basic auth
            // In production, you'd implement proper HMAC signature
            request = request.header("KALSHI-ACCESS-KEY", key_id);
        }
    }

    let response = request.send().await?;

    if !response.status().is_success() {
        return Err(KalshiError::ParseError(format!(
            "API returned status: {}",
            response.status()
        )));
    }

    let text = response.text().await?;
    
    match serde_json::from_str::<TradesResponse>(&text) {
        Ok(response) => Ok(response.trades),
        Err(e) => {
            // If parsing fails, return empty list to allow tool to continue
            eprintln!("Warning: Failed to parse Kalshi response: {}", e);
            Ok(Vec::new())
        }
    }
}

#[derive(Debug, Deserialize)]
struct MarketResponse {
    market: MarketData,
}

#[derive(Debug, Deserialize)]
struct MarketData {
    title: Option<String>,
    subtitle: Option<String>,
}

pub async fn fetch_market_info(ticker: &str) -> Option<String> {
    let client = reqwest::Client::new();
    let url = format!("https://api.elections.kalshi.com/trade-api/v2/markets/{}", ticker);
    
    match client.get(&url).send().await {
        Ok(response) if response.status().is_success() => {
            if let Ok(text) = response.text().await {
                if let Ok(market_response) = serde_json::from_str::<MarketResponse>(&text) {
                    return market_response.market.title
                        .or(market_response.market.subtitle);
                }
            }
        }
        _ => {}
    }
    
    None
}
