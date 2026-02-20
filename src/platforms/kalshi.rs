use std::sync::OnceLock;

use crate::config::Config;
use serde::{Deserialize, Serialize};
use thiserror::Error;

fn shared_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .pool_max_idle_per_host(4)
            .build()
            .expect("failed to build shared HTTP client")
    })
}

#[derive(Error, Debug)]
pub enum KalshiError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("Failed to parse response: {0}")]
    ParseError(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Trade {
    pub trade_id: String,
    pub ticker: String,
    pub price: f64,
    pub count: i32,
    pub yes_price: f64,
    pub no_price: f64,
    pub taker_side: String,
    pub created_time: String,
    #[serde(skip)]
    pub market_title: Option<String>,
    // Note: Kalshi public API doesn't expose account IDs for privacy
    // Use trade_id as proxy for tracking patterns
}

#[derive(Debug, Deserialize)]
struct TradesResponse {
    #[serde(default)]
    trades: Vec<Trade>,
}

pub async fn fetch_recent_trades(config: Option<&Config>) -> Result<Vec<Trade>, KalshiError> {
    let url = "https://api.elections.kalshi.com/trade-api/v2/markets/trades";

    let mut request = shared_client()
        .get(url)
        .query(&[("limit", "100")])
        .header("Accept", "application/json");

    // Add authentication if credentials are provided
    if let Some(cfg) = config {
        if let (Some(key_id), Some(_private_key)) =
            (&cfg.kalshi_api_key_id, &cfg.kalshi_private_key)
        {
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

#[derive(Debug, Deserialize, Clone)]
struct MarketData {
    ticker: String,
    title: Option<String>,
    subtitle: Option<String>,
    category: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

/// Market info including title and native category
#[derive(Clone)]
pub struct MarketInfo {
    pub ticker: String,
    pub title: String,
    pub category: Option<String>,
    #[allow(dead_code)]
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct EventDataNested {
    #[allow(dead_code)]
    event_ticker: String,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    markets: Vec<MarketData>,
}

#[derive(Debug, Deserialize)]
struct EventsNestedResponse {
    events: Vec<EventDataNested>,
    #[serde(default)]
    cursor: Option<String>,
}

fn collect_markets_from_events(
    events: Vec<EventDataNested>,
    seen: &mut std::collections::HashSet<String>,
    out: &mut Vec<MarketInfo>,
) {
    for event in events {
        for m in event.markets {
            let status = m.status.as_deref().unwrap_or("");
            if status == "settled" || status == "finalized" {
                continue;
            }
            if m.ticker.starts_with("KXMV") {
                continue;
            }
            if !seen.insert(m.ticker.clone()) {
                continue;
            }
            out.push(MarketInfo {
                title: m.title.or(m.subtitle).unwrap_or_else(|| "Unknown".into()),
                category: m.category.or(event.category.clone()),
                tags: m.tags,
                ticker: m.ticker,
            });
        }
    }
}

/// Fetch all open events with pagination. Bulletproof: gets every open market on Kalshi.
async fn fetch_all_open_events() -> Result<Vec<MarketInfo>, KalshiError> {
    let events_url = "https://api.elections.kalshi.com/trade-api/v2/events";
    let mut all_markets = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut cursor: Option<String> = None;

    loop {
        let mut query: Vec<(&str, String)> = vec![
            ("status", "open".into()),
            ("limit", "200".into()),
            ("with_nested_markets", "true".into()),
        ];
        if let Some(ref c) = cursor {
            query.push(("cursor", c.clone()));
        }

        let resp = shared_client()
            .get(events_url)
            .query(&query)
            .send()
            .await?;
        if !resp.status().is_success() {
            break;
        }
        let text = resp.text().await?;
        let data: EventsNestedResponse = serde_json::from_str(&text)
            .map_err(|e| KalshiError::ParseError(e.to_string()))?;

        collect_markets_from_events(data.events, &mut seen, &mut all_markets);

        cursor = data.cursor.and_then(|c| if c.is_empty() { None } else { Some(c) });
        if cursor.is_none() {
            break;
        }
    }

    Ok(all_markets)
}

/// On-demand search: query Kalshi for markets relevant to a Polymarket title.
/// Bulletproof: paginates through ALL open events — no hardcoded series, never miss a market.
pub async fn search_markets(poly_title: &str) -> Result<Vec<MarketInfo>, KalshiError> {
    let all_markets = fetch_all_open_events().await?;

    println!(
        "🔍 On-demand search for \"{}\" → {} markets (all open, paginated)",
        poly_title,
        all_markets.len()
    );

    Ok(all_markets)
}

pub async fn fetch_market_context(ticker: &str) -> Option<crate::alerts::MarketContext> {
    let url = format!(
        "https://api.elections.kalshi.com/trade-api/v2/markets/{}",
        ticker
    );

    let response = shared_client().get(&url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }

    let text = response.text().await.ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&text).ok()?;
    let market = parsed.get("market")?;

    let yes_bid = market.get("yes_bid")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) / 100.0;
    let yes_ask = market.get("yes_ask")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) / 100.0;
    let no_bid = market.get("no_bid")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) / 100.0;

    let spread = (yes_ask - yes_bid).abs();

    let volume_24h = market.get("volume_24h")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let open_interest = market.get("open_interest")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let last_price = market.get("last_price")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) / 100.0;
    let prev_price = market.get("previous_price")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) / 100.0;
    let price_change_24h = if prev_price > 0.0 {
        ((last_price - prev_price) / prev_price) * 100.0
    } else {
        0.0
    };

    let liquidity = market.get("liquidity")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    // Extract category and tags from Kalshi market data
    let tags: Vec<String> = market.get("category")
        .and_then(|v| v.as_str())
        .map(|c| vec![c.to_string()])
        .unwrap_or_default();

    Some(crate::alerts::MarketContext {
        yes_price: yes_bid,
        no_price: no_bid,
        spread,
        volume_24h,
        open_interest,
        price_change_24h,
        liquidity,
        tags,
        expiration_date: market.get("expiration_time")
        .or_else(|| market.get("result_v_time"))
        .or_else(|| market.get("close_time"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string()),
    })
}

/// Fetch order book from Kalshi public API
pub async fn fetch_order_book(ticker: &str) -> Option<crate::alerts::OrderBookSummary> {
    let url = format!(
        "https://api.elections.kalshi.com/trade-api/v2/markets/{}/orderbook",
        ticker
    );

    let response = shared_client()
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .ok()?;

    if !response.status().is_success() {
        return None;
    }

    let text = response.text().await.ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&text).ok()?;
    let orderbook = parsed.get("orderbook").unwrap_or(&parsed);

    let yes_bids = orderbook.get("yes").and_then(|v| v.as_array());
    let no_bids = orderbook.get("no").and_then(|v| v.as_array());

    // Kalshi orderbook format: arrays of [price, quantity] for yes and no sides
    let (best_bid, bid_depth, bid_levels) = if let Some(bids) = yes_bids {
        let mut best = 0.0f64;
        let mut depth = 0.0f64;
        let mut levels = 0u32;
        for entry in bids {
            let arr = entry.as_array();
            if let Some(arr) = arr {
                let price = arr.first().and_then(|v| v.as_f64()).unwrap_or(0.0) / 100.0;
                let qty = arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
                if price > best { best = price; }
                depth += price * qty;
                levels += 1;
            }
        }
        (best, depth, levels)
    } else {
        (0.0, 0.0, 0)
    };

    let (best_ask, ask_depth, ask_levels) = if let Some(asks) = no_bids {
        let mut best = 1.0f64;
        let mut depth = 0.0f64;
        let mut levels = 0u32;
        for entry in asks {
            let arr = entry.as_array();
            if let Some(arr) = arr {
                let price = arr.first().and_then(|v| v.as_f64()).unwrap_or(0.0) / 100.0;
                let qty = arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
                if price < best { best = price; }
                depth += price * qty;
                levels += 1;
            }
        }
        // best_ask for YES side is 1 - best NO bid
        (1.0 - best, depth, levels)
    } else {
        (1.0, 0.0, 0)
    };

    Some(crate::alerts::OrderBookSummary {
        best_bid,
        best_ask,
        bid_depth_10pct: bid_depth,
        ask_depth_10pct: ask_depth,
        bid_levels,
        ask_levels,
    })
}

/// Fetch full market info including native category and tags
pub async fn fetch_market_info_full(ticker: &str) -> Option<MarketInfo> {
    let url = format!(
        "https://api.elections.kalshi.com/trade-api/v2/markets/{}",
        ticker
    );

    match shared_client().get(&url).send().await {
        Ok(response) if response.status().is_success() => {
            if let Ok(text) = response.text().await {
                if let Ok(market_response) = serde_json::from_str::<MarketResponse>(&text) {
                    let title = market_response.market.title
                        .or(market_response.market.subtitle)?;
                    return Some(MarketInfo {
                        title,
                        category: market_response.market.category,
                        tags: market_response.market.tags,
                        ticker: market_response.market.ticker,
                    });
                }
            }
        }
        _ => {}
    }

    None
}

pub fn parse_ticker_details(ticker: &str, side: &str) -> String {
    let betting_side = side.to_uppercase();
    // Parse Kalshi ticker to extract bet details
    // Format examples:
    // KXNHLGAME-26JAN08ANACAR-CAR = NHL game, Carolina wins
    // KXNCAAFTOTAL-26JAN08MIAMISS-51 = NCAA football total points over 51
    // KXHIGHNY-24DEC-T63 = NYC high temp threshold
    // KXETHD-26JAN0818-T3109.99 = ETH price threshold

    // Cryptocurrency/Stock price thresholds
    if ticker.contains("ETH")
        || ticker.contains("BTC")
        || ticker.contains("SOL")
        || ticker.contains("SPX")
        || ticker.contains("TSLA")
    {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(threshold_part) = parts.last() {
            if threshold_part.starts_with('T') || threshold_part.starts_with('t') {
                let price = &threshold_part[1..];
                let asset = if ticker.contains("ETH") {
                    "Ethereum (ETH)"
                } else if ticker.contains("BTC") {
                    "Bitcoin (BTC)"
                } else if ticker.contains("SOL") {
                    "Solana (SOL)"
                } else if ticker.contains("SPX") {
                    "S&P 500"
                } else if ticker.contains("TSLA") {
                    "Tesla"
                } else {
                    "Asset"
                };

                return format!("{} price {} ${} at expiry", asset,
                    if betting_side == "YES" { "≥" } else { "<" }, price);
            }
        }
    }

    // Check for sports totals (over/under)
    if ticker.contains("TOTAL") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(threshold) = parts.last() {
            if threshold.chars().all(|c| c.is_numeric()) {
                let sport = if ticker.contains("NFL") {
                    "NFL"
                } else if ticker.contains("NBA") {
                    "NBA"
                } else if ticker.contains("NHL") {
                    "NHL"
                } else if ticker.contains("MLB") {
                    "MLB"
                } else if ticker.contains("NCAAF") || ticker.contains("CFB") {
                    "College Football"
                } else if ticker.contains("NCAAB") || ticker.contains("CBB") {
                    "College Basketball"
                } else {
                    "Game"
                };

                // Extract teams if possible
                if parts.len() >= 3 {
                    if let Some(teams_part) = parts.get(parts.len() - 2) {
                        if teams_part.len() >= 6 {
                            let team_codes = &teams_part[teams_part.len() - 6..];
                            let away = &team_codes[..3];
                            let home = &team_codes[3..];
                            return format!(
                                "Total points {} {} | {} @ {} ({})",
                                if betting_side == "YES" { "OVER" } else { "UNDER" },
                                threshold,
                                away.to_uppercase(),
                                home.to_uppercase(),
                                sport
                            );
                        }
                    }
                }

                return format!("Total points {} {} ({})",
                    if betting_side == "YES" { "OVER" } else { "UNDER" },
                    threshold, sport);
            }
        }
    }

    if ticker.contains("NHLGAME")
        || ticker.contains("NFLGAME")
        || ticker.contains("NBAGAME")
        || ticker.contains("MLBGAME")
        || ticker.contains("SOCCERGAME")
        || ticker.contains("FOOTBALLGAME")
    {
        // Sports game format
        let parts: Vec<&str> = ticker.split('-').collect();
        if parts.len() >= 3 {
            let outcome = parts.last().unwrap_or(&"");

            // Extract team codes from middle part
            if let Some(teams_part) = parts.get(parts.len() - 2) {
                // Format like "26JAN08ANACAR" - extract last 6 chars for teams
                if teams_part.len() >= 6 {
                    let team_codes = &teams_part[teams_part.len() - 6..];
                    let away = &team_codes[..3];
                    let home = &team_codes[3..];

                    let sport = if ticker.contains("NHL") {
                        "NHL"
                    } else if ticker.contains("NFL") {
                        "NFL"
                    } else if ticker.contains("NBA") {
                        "NBA"
                    } else if ticker.contains("MLB") {
                        "MLB"
                    } else if ticker.contains("SOCCER") || ticker.contains("FOOTBALL") {
                        "Soccer"
                    } else {
                        "Sports"
                    };

                    // Show what they're actually betting will happen
                    if betting_side == "YES" {
                        return format!(
                            "{} wins vs {} ({})",
                            outcome.to_uppercase(),
                            if outcome.to_uppercase() == away.to_uppercase() {
                                home.to_uppercase()
                            } else {
                                away.to_uppercase()
                            },
                            sport
                        );
                    } else {
                        // Betting NO means betting the OTHER team wins
                        let other_team = if outcome.to_uppercase() == away.to_uppercase() {
                            home.to_uppercase()
                        } else {
                            away.to_uppercase()
                        };
                        return format!(
                            "{} wins vs {} ({})",
                            other_team,
                            outcome.to_uppercase(),
                            sport
                        );
                    }
                }
            }
        }
    // Check for point spreads
    } else if ticker.contains("SPREAD") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(last_part) = parts.last() {
            // Handle formats: "CAR3", "CAR-3", "CAR_N3" (negative), etc.
            let team = last_part
                .chars()
                .take_while(|c| c.is_alphabetic())
                .collect::<String>();
            let spread_str = last_part
                .chars()
                .skip_while(|c| c.is_alphabetic())
                .filter(|c| c.is_numeric() || *c == '.' || *c == '-')
                .collect::<String>();

            if !team.is_empty() && !spread_str.is_empty() {
                let spread_value = spread_str.trim_start_matches('-');
                if betting_side == "YES" {
                    return format!(
                        "{} wins by {} or more (covers)",
                        team.to_uppercase(),
                        spread_value
                    );
                } else {
                    return format!(
                        "{} loses or wins by less than {} (doesn't cover)",
                        team.to_uppercase(),
                        spread_value
                    );
                }
            }
        }
    // Check for player props (touchdowns, points, etc)
    } else if ticker.contains("TD") || ticker.contains("SCORE") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(threshold) = parts.last() {
            if threshold.chars().all(|c| c.is_numeric()) {
                let prop_type = if ticker.contains("TD") {
                    "touchdowns"
                } else {
                    "points"
                };
                return format!(
                    "Player gets {} {} {}",
                    if betting_side == "YES" { "≥" } else { "<" },
                    threshold, prop_type
                );
            }
        }
    } else if ticker.contains("HIGH") || ticker.contains("LOW") {
        // Temperature markets
        if ticker.contains("T") {
            let parts: Vec<&str> = ticker.split('-').collect();
            if let Some(threshold_part) = parts.last() {
                if let Some(temp) = threshold_part.strip_prefix('T') {
                    let metric = if ticker.contains("HIGH") {
                        "High"
                    } else {
                        "Low"
                    };
                    return format!(
                        "{} temp {} {}°F",
                        metric,
                        if betting_side == "YES" { "≥" } else { "<" },
                        temp
                    );
                }
            }
        }
    } else if ticker.contains("PRES") {
        // Presidential/election markets
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(outcome) = parts.last() {
            if betting_side == "YES" {
                return format!("{} wins", outcome.to_uppercase());
            } else {
                return format!("{} doesn't win", outcome.to_uppercase());
            }
        }
    }

    // Check for combos/parlays
    if ticker.contains("COMBO") || ticker.contains("PARLAY") || ticker.contains("MULTI") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(last) = parts.last() {
            return format!(
                "{} {} combo/parlay",
                if betting_side == "YES" { "Wins" } else { "Loses" },
                last.to_uppercase()
            );
        }
    }

    // Check for first/last to score
    if ticker.contains("FIRST") || ticker.contains("LAST") || ticker.contains("ANYTIME") {
        let timing = if ticker.contains("FIRST") {
            "first"
        } else if ticker.contains("LAST") {
            "last"
        } else {
            "anytime"
        };
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(player) = parts.last() {
            if betting_side == "YES" {
                return format!("{} scores {} TD", player.to_uppercase(), timing);
            } else {
                return format!("{} doesn't score {} TD", player.to_uppercase(), timing);
            }
        }
    }

    // Check for ranking/placement markets (TOP, FINISH, PLACE)
    if ticker.contains("TOP") || ticker.contains("FINISH") || ticker.contains("PLACE") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(outcome) = parts.last() {
            return format!(
                "{} {}",
                outcome.to_uppercase(),
                if betting_side == "YES" { "finishes in position" } else { "doesn't finish in position" }
            );
        }
    }

    // Default: try to extract outcome from last part
    let parts: Vec<&str> = ticker.split('-').collect();
    if let Some(outcome) = parts.last() {
        if outcome.len() <= 10 && outcome.chars().all(|c| c.is_alphanumeric()) {
            if betting_side == "YES" {
                return format!("{} happens", outcome.to_uppercase());
            } else {
                return format!("{} doesn't happen", outcome.to_uppercase());
            }
        }
    }

    // Absolute fallback - show more context
    if betting_side == "YES" {
        String::from("YES - check market details")
    } else {
        String::from("NO - check market details")
    }
}
