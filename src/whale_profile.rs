use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

fn shared_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(4)
            .build()
            .expect("failed to build shared HTTP client")
    })
}

const PROFILE_TTL: Duration = Duration::from_secs(30 * 60); // 30 min cache
const LEADERBOARD_TTL: Duration = Duration::from_secs(60 * 60); // 1 hour cache

/// Whale intelligence data fetched from Polymarket Data API
#[derive(Debug, Clone, Serialize)]
pub struct WhaleProfile {
    pub wallet_id: String,
    pub portfolio_value: Option<f64>,
    pub positions_count: Option<u32>,
    pub leaderboard_rank: Option<u32>,
    pub leaderboard_profit: Option<f64>,
    pub win_rate: Option<f64>,
    pub markets_traded: Option<u32>,
}

/// Cached whale profiles + leaderboard
pub struct WhaleProfileCache {
    profiles: HashMap<String, (WhaleProfile, Instant)>,
    leaderboard: Option<(Vec<LeaderboardEntry>, Instant)>,
}

#[derive(Debug, Clone, Deserialize)]
struct LeaderboardEntry {
    #[serde(rename = "proxyWallet", default)]
    proxy_wallet: Option<String>,
    #[serde(default)]
    rank: serde_json::Value, // Can be string or number
    #[serde(rename = "pnl", default)]
    pnl: Option<f64>,
    #[serde(rename = "vol", default)]
    #[allow(dead_code)]
    vol: Option<f64>,
    #[serde(rename = "marketsTraded", default)]
    markets_traded: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct PositionEntry {
    #[serde(rename = "currentValue", default)]
    #[allow(dead_code)]
    current_value: Option<f64>,
    #[serde(default)]
    #[allow(dead_code)]
    size: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct ClosedPositionEntry {
    #[serde(rename = "realizedPnl", default)]
    realized_pnl: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct ValueResponse {
    #[serde(rename = "value", default)]
    value: Option<f64>,
}

impl WhaleProfileCache {
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
            leaderboard: None,
        }
    }

    /// Get a cached whale profile if still fresh
    pub fn get(&self, wallet_id: &str) -> Option<&WhaleProfile> {
        let (profile, fetched_at) = self.profiles.get(wallet_id)?;
        if fetched_at.elapsed() < PROFILE_TTL {
            Some(profile)
        } else {
            None
        }
    }

    /// Store a whale profile in cache
    pub fn insert(&mut self, profile: WhaleProfile) {
        let key = profile.wallet_id.clone();
        self.profiles.insert(key, (profile, Instant::now()));
    }

    /// Look up wallet in the cached leaderboard
    pub fn leaderboard_lookup(&self, wallet_id: &str) -> Option<(u32, f64, Option<u32>)> {
        let (entries, fetched_at) = self.leaderboard.as_ref()?;
        if fetched_at.elapsed() >= LEADERBOARD_TTL {
            return None;
        }
        let lower = wallet_id.to_lowercase();
        for entry in entries {
            if let Some(ref pw) = entry.proxy_wallet {
                if pw.to_lowercase() == lower {
                    let rank_val = match &entry.rank {
                        serde_json::Value::Number(n) => n.as_u64().map(|v| v as u32).unwrap_or(0),
                        serde_json::Value::String(s) => s.parse().unwrap_or(0),
                        _ => 0,
                    };
                    return Some((
                        rank_val,
                        entry.pnl.unwrap_or(0.0),
                        entry.markets_traded,
                    ));
                }
            }
        }
        None
    }

    /// Refresh leaderboard cache if stale
    pub async fn refresh_leaderboard_if_needed(&mut self) {
        let needs_refresh = match &self.leaderboard {
            None => true,
            Some((_, fetched_at)) => fetched_at.elapsed() >= LEADERBOARD_TTL,
        };

        if needs_refresh {
            if let Some(entries) = fetch_leaderboard().await {
                self.leaderboard = Some((entries, Instant::now()));
            }
        }
    }

    /// Clean expired entries
    pub fn prune(&mut self) {
        self.profiles.retain(|_, (_, fetched_at)| fetched_at.elapsed() < PROFILE_TTL);
    }
}

/// Fetch trader leaderboard (top 500)
async fn fetch_leaderboard() -> Option<Vec<LeaderboardEntry>> {
    let response = shared_client()
        .get("https://data-api.polymarket.com/v1/leaderboard")
        .query(&[("limit", "500")])
        .header("Accept", "application/json")
        .send()
        .await
        .ok()?;

    if !response.status().is_success() {
        return None;
    }

    let text = response.text().await.ok()?;
    serde_json::from_str(&text).ok()
}

/// Fetch portfolio total value for a wallet
async fn fetch_portfolio_value(wallet_id: &str) -> Option<f64> {
    let response = shared_client()
        .get("https://data-api.polymarket.com/value")
        .query(&[("user", wallet_id)])
        .header("Accept", "application/json")
        .send()
        .await
        .ok()?;

    if !response.status().is_success() {
        return None;
    }

    let text = response.text().await.ok()?;

    // The response might be a direct number, an object, or an array of objects
    if let Ok(val) = text.trim().parse::<f64>() {
        return Some(val);
    }
    if let Ok(resp) = serde_json::from_str::<ValueResponse>(&text) {
        return resp.value;
    }
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
        if let Some(arr) = v.as_array() {
            if let Some(first) = arr.first() {
                return first.get("value")
                    .or_else(|| first.get("totalValue"))
                    .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())));
            }
        }
        return v.get("value")
            .or_else(|| v.get("totalValue"))
            .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())));
    }
    None
}

/// Fetch current open positions count
async fn fetch_positions_count(wallet_id: &str) -> Option<u32> {
    let response = shared_client()
        .get("https://data-api.polymarket.com/positions")
        .query(&[("user", wallet_id), ("limit", "100")])
        .header("Accept", "application/json")
        .send()
        .await
        .ok()?;

    if !response.status().is_success() {
        return None;
    }

    let text = response.text().await.ok()?;
    let positions: Vec<PositionEntry> = serde_json::from_str(&text).ok()?;
    Some(positions.len() as u32)
}

/// Compute win rate from ALL closed positions (paginated — API returns 50 per page sorted by PnL desc).
async fn fetch_win_rate(wallet_id: &str) -> Option<(f64, u32)> {
    let mut all_positions: Vec<ClosedPositionEntry> = Vec::new();
    let page_size = 50;

    for page in 0..20 {
        let offset = page * page_size;
        let resp = shared_client()
            .get("https://data-api.polymarket.com/closed-positions")
            .query(&[
                ("user", wallet_id),
                ("limit", &page_size.to_string()),
                ("offset", &offset.to_string()),
            ])
            .header("Accept", "application/json")
            .send()
            .await
            .ok()?;

        if !resp.status().is_success() {
            break;
        }

        let text = resp.text().await.ok()?;
        let page_positions: Vec<ClosedPositionEntry> = serde_json::from_str(&text).ok()?;
        let count = page_positions.len();
        all_positions.extend(page_positions);

        if count < page_size {
            break;
        }
    }

    if all_positions.is_empty() {
        return None;
    }

    let total = all_positions.len() as u32;
    let wins = all_positions
        .iter()
        .filter(|p| p.realized_pnl.unwrap_or(0.0) > 0.0)
        .count() as u32;

    let rate = if total > 0 {
        wins as f64 / total as f64
    } else {
        0.0
    };
    Some((rate, total))
}

/// Fetch full whale profile for a Polymarket wallet (3 parallel API calls + leaderboard lookup)
pub async fn fetch_whale_profile(wallet_id: &str, cache: &mut WhaleProfileCache) -> Option<WhaleProfile> {
    // Check cache first
    if let Some(cached) = cache.get(wallet_id) {
        return Some(cached.clone());
    }

    // Refresh leaderboard if needed
    cache.refresh_leaderboard_if_needed().await;

    // Fetch portfolio data in parallel
    let (value, positions, win_data) = tokio::join!(
        fetch_portfolio_value(wallet_id),
        fetch_positions_count(wallet_id),
        fetch_win_rate(wallet_id),
    );

    // Look up in leaderboard
    let lb = cache.leaderboard_lookup(wallet_id);

    let (win_rate, markets_traded) = match win_data {
        Some((rate, count)) => (Some(rate), Some(count)),
        None => (None, lb.and_then(|(_, _, mt)| mt)),
    };

    let profile = WhaleProfile {
        wallet_id: wallet_id.to_string(),
        portfolio_value: value,
        positions_count: positions,
        leaderboard_rank: lb.map(|(rank, _, _)| rank),
        leaderboard_profit: lb.map(|(_, profit, _)| profit),
        win_rate,
        markets_traded,
    };

    // Only cache if we got at least some data
    if profile.portfolio_value.is_some() || profile.leaderboard_rank.is_some() || profile.win_rate.is_some() {
        cache.insert(profile.clone());
    }

    Some(profile)
}
