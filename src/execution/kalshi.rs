use base64::{engine::general_purpose, Engine as _};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use rsa::{
    pkcs8::DecodePrivateKey,
    pkcs1::DecodeRsaPrivateKey,
    pss::BlindedSigningKey,
    sha2::Sha256,
    signature::{RandomizedSigner, SignatureEncoding},
    RsaPrivateKey,
};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct KalshiExecutor {
    client: reqwest::Client,
    base_url: String,
    key_id: String,
    signing_key: BlindedSigningKey<Sha256>,
}

#[derive(Serialize)]
struct CreateOrderRequest {
    ticker: String,
    action: String,
    #[serde(rename = "type")]
    order_type: String,
    side: String,
    count: i32,
    yes_price: Option<i64>,
    no_price: Option<i64>,
    client_order_id: String,
}

#[derive(Deserialize, Debug)]
struct OrderResponse {
    order: OrderObj,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct OrderObj {
    order_id: String,
    status: String,
    #[serde(default)]
    fill_count: i32,
    #[serde(default)]
    remaining_count: i32,
}

impl KalshiExecutor {
    pub fn new(key_id: String, private_key_pem: &str, is_demo: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let private_key = RsaPrivateKey::from_pkcs8_pem(private_key_pem)
            .or_else(|_| RsaPrivateKey::from_pkcs1_pem(private_key_pem))?;
        let signing_key = BlindedSigningKey::<Sha256>::new(private_key);
        let base_url = if is_demo {
            "https://demo-api.kalshi.co/trade-api/v2".to_string()
        } else {
            "https://api.elections.kalshi.com/trade-api/v2".to_string()
        };

        Ok(Self {
            client: reqwest::Client::new(),
            base_url,
            key_id,
            signing_key,
        })
    }

    fn sign_request(&self, method: &str, path: &str, timestamp: &str) -> Result<String, Box<dyn std::error::Error>> {
        let msg = format!("{}{}{}", timestamp, method, path);
        let mut rng = rand::thread_rng();
        let signature = self.signing_key.sign_with_rng(&mut rng, msg.as_bytes());
        Ok(general_purpose::STANDARD.encode(signature.to_bytes()))
    }

    fn auth_headers(&self, method: &str, path: &str) -> Result<HeaderMap, Box<dyn std::error::Error>> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_millis()
            .to_string();
        let signature = self.sign_request(method, path, &timestamp)?;
        let mut headers = HeaderMap::new();
        headers.insert("KALSHI-ACCESS-KEY", HeaderValue::from_str(&self.key_id)?);
        headers.insert("KALSHI-ACCESS-SIGNATURE", HeaderValue::from_str(&signature)?);
        headers.insert("KALSHI-ACCESS-TIMESTAMP", HeaderValue::from_str(&timestamp)?);
        Ok(headers)
    }

    pub async fn get_balance(&self) -> Result<i64, Box<dyn std::error::Error>> {
        let path = "/trade-api/v2/portfolio/balance";
        let url = format!("{}/portfolio/balance", self.base_url);
        let headers = self.auth_headers("GET", path)?;

        let resp = self.client
            .get(&url)
            .headers(headers)
            .send()
            .await?;

        if resp.status().is_success() {
            let data: serde_json::Value = resp.json().await?;
            let balance_cents = data.get("balance")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            Ok(balance_cents)
        } else {
            let err_text = resp.text().await?;
            Err(format!("Balance check failed: {}", err_text).into())
        }
    }

    pub async fn has_open_position(&self, event_ticker: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let path = "/trade-api/v2/portfolio/positions";
        let url = format!("{}/portfolio/positions", self.base_url);
        let headers = self.auth_headers("GET", path)?;

        let resp = self.client
            .get(&url)
            .headers(headers)
            .query(&[
                ("event_ticker", event_ticker),
                ("count_filter", "position"),
                ("limit", "10"),
            ])
            .send()
            .await?;

        if resp.status().is_success() {
            let data: serde_json::Value = resp.json().await?;
            let positions = data.get("market_positions")
                .or_else(|| data.get("positions"))
                .and_then(|v| v.as_array());
            match positions {
                Some(arr) => Ok(!arr.is_empty()),
                None => Ok(false),
            }
        } else {
            let err_text = resp.text().await?;
            Err(format!("Position check failed: {}", err_text).into())
        }
    }

    pub async fn get_open_event_tickers(&self) -> Result<std::collections::HashSet<String>, Box<dyn std::error::Error>> {
        let path = "/trade-api/v2/portfolio/positions";
        let url = format!("{}/portfolio/positions", self.base_url);
        let headers = self.auth_headers("GET", path)?;

        let resp = self.client
            .get(&url)
            .headers(headers)
            .query(&[
                ("count_filter", "position"),
                ("limit", "1000"),
            ])
            .send()
            .await?;

        let mut event_tickers = std::collections::HashSet::new();
        if resp.status().is_success() {
            let data: serde_json::Value = resp.json().await?;
            if let Some(positions) = data.get("market_positions")
                .or_else(|| data.get("positions"))
                .and_then(|v| v.as_array())
            {
                for pos in positions {
                    if let Some(ticker) = pos.get("ticker").and_then(|v| v.as_str()) {
                        let event_key = match ticker.rfind('-') {
                            Some(p) => ticker[..p].to_string(),
                            None => ticker.to_string(),
                        };
                        event_tickers.insert(event_key);
                    }
                    if let Some(et) = pos.get("event_ticker").and_then(|v| v.as_str()) {
                        event_tickers.insert(et.to_string());
                    }
                }
            }
        }
        Ok(event_tickers)
    }

    /// Fetch all open positions with ticker, side, and count. For display/verification.
    pub async fn get_positions(&self) -> Result<Vec<(String, String, i32)>, Box<dyn std::error::Error>> {
        let path = "/trade-api/v2/portfolio/positions";
        let url = format!("{}/portfolio/positions", self.base_url);
        let headers = self.auth_headers("GET", path)?;

        let resp = self.client
            .get(&url)
            .headers(headers)
            .query(&[
                ("count_filter", "position"),
                ("limit", "1000"),
            ])
            .send()
            .await?;

        if !resp.status().is_success() {
            let err_text = resp.text().await?;
            return Err(format!("Positions API failed: {}", err_text).into());
        }

        let data: serde_json::Value = resp.json().await?;
        let mut out = Vec::new();
        if let Some(positions) = data.get("market_positions")
            .or_else(|| data.get("positions"))
            .and_then(|v| v.as_array())
        {
            for pos in positions {
                let ticker = pos.get("ticker").and_then(|v| v.as_str()).unwrap_or("?").to_string();
                let pos_val = pos.get("position")
                    .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
                    .unwrap_or(0) as i32;
                if pos_val == 0 {
                    continue;
                }
                let side = if pos_val > 0 { "YES" } else { "NO" };
                let count = pos_val.abs();
                out.push((ticker, side.to_string(), count));
            }
        }
        Ok(out)
    }

    pub async fn place_order(
        &self,
        ticker: &str,
        side: &str,
        count: i32,
        price_cents: i64,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let path = "/trade-api/v2/portfolio/orders";
        let url = format!("{}/portfolio/orders", self.base_url);
        let mut headers = self.auth_headers("POST", path)?;
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let body = CreateOrderRequest {
            ticker: ticker.to_string(),
            action: "buy".to_string(),
            order_type: "limit".to_string(),
            side: side.to_lowercase(),
            count,
            yes_price: if side.to_lowercase() == "yes" { Some(price_cents) } else { None },
            no_price: if side.to_lowercase() == "no" { Some(price_cents) } else { None },
            client_order_id: uuid::Uuid::new_v4().to_string(),
        };

        let resp = self.client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await?;

        if resp.status().is_success() {
            let order_resp: OrderResponse = resp.json().await?;
            println!("✅ ORDER PLACED: {} {} @ {}c (ID: {})", side.to_uppercase(), ticker, price_cents, order_resp.order.order_id);
            Ok(order_resp.order.order_id)
        } else {
            let err_text = resp.text().await?;
            eprintln!("❌ ORDER FAILED: {}", err_text);
            Err(format!("API Error: {}", err_text).into())
        }
    }

    /// Fetch order status. Returns (status, fill_count). Used to verify fills before counting against daily loss.
    pub async fn get_order_status(&self, order_id: &str) -> Result<(String, i32), Box<dyn std::error::Error>> {
        let path = format!("/trade-api/v2/portfolio/orders/{}", order_id);
        let url = format!("{}/portfolio/orders/{}", self.base_url, order_id);
        let headers = self.auth_headers("GET", &path)?;

        let resp = self.client
            .get(&url)
            .headers(headers)
            .send()
            .await?;

        if resp.status().is_success() {
            let data: serde_json::Value = resp.json().await?;
            let order = data.get("order").ok_or("Missing order in response")?;
            let status = order.get("status").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
            let fill_count = order.get("fill_count")
                .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
                .unwrap_or(0) as i32;
            Ok((status, fill_count))
        } else {
            let err_text = resp.text().await?;
            Err(format!("Order status check failed: {}", err_text).into())
        }
    }
}
