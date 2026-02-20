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
    private_key: RsaPrivateKey,
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
struct OrderObj {
    order_id: String,
    status: String,
}

impl KalshiExecutor {
    pub fn new(key_id: String, private_key_pem: &str, is_demo: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let private_key = RsaPrivateKey::from_pkcs8_pem(private_key_pem)
            .or_else(|_| RsaPrivateKey::from_pkcs1_pem(private_key_pem))?;
        let base_url = if is_demo {
            "https://demo-api.kalshi.co/trade-api/v2".to_string()
        } else {
            "https://api.elections.kalshi.com/trade-api/v2".to_string()
        };

        Ok(Self {
            client: reqwest::Client::new(),
            base_url,
            key_id,
            private_key,
        })
    }

    fn sign_request(&self, method: &str, path: &str, timestamp: &str) -> Result<String, Box<dyn std::error::Error>> {
        let msg = format!("{}{}{}", timestamp, method, path);
        let mut rng = rand::thread_rng();
        let signing_key = BlindedSigningKey::<Sha256>::new(self.private_key.clone());
        let signature = signing_key.sign_with_rng(&mut rng, msg.as_bytes());
        Ok(general_purpose::STANDARD.encode(signature.to_bytes()))
    }

    pub async fn place_order(
        &self,
        ticker: &str,
        side: &str,
        count: i32,
        price_cents: i64,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let path = "/portfolio/orders";
        let url = format!("{}{}", self.base_url, path);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_millis()
            .to_string();

        let signature = self.sign_request("POST", path, &timestamp)?;

        let mut headers = HeaderMap::new();
        headers.insert("KALSHI-ACCESS-KEY", HeaderValue::from_str(&self.key_id)?);
        headers.insert("KALSHI-ACCESS-SIGNATURE", HeaderValue::from_str(&signature)?);
        headers.insert("KALSHI-ACCESS-TIMESTAMP", HeaderValue::from_str(&timestamp)?);
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
}
