use colored::*;
use serde_json::json;

/// Sanitize text for messaging platforms that use Markdown/HTML parsing
pub fn escape_special_chars(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | ' ' | ',' | ':' | '?' | '.' => c,
            '(' | '[' | '{' => '(',
            ')' | ']' | '}' => ')',
            _ => ' ',
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

/// Info about an executed Kalshi trade, used to build rich Discord embeds.
pub struct ExecutionAlert {
    pub kalshi_ticker: String,
    pub side: String,
    pub count: i32,
    pub price_cents: i64,
    pub fee_cents: i64,
    pub total_cost_cents: i64,
    pub ev_cents: f64,
    pub kelly_pct: f64,
    pub whale_win_rate: f64,
    pub balance_after_cents: i64,
    pub poly_title: String,
    pub order_id: String,
}

/// Send a rich Discord embed for an executed Kalshi trade.
pub async fn send_execution_alert(webhook_url: &str, alert: &ExecutionAlert) {
    let is_discord = webhook_url.contains("discord.com/api/webhooks");

    let payload = if is_discord {
        build_discord_embed(alert)
    } else {
        build_generic_payload(alert)
    };

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "{} Failed to create webhook client: {}",
                "[WEBHOOK ERROR]".red(),
                e
            );
            return;
        }
    };

    match client.post(webhook_url).json(&payload).send().await {
        Ok(response) => {
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                eprintln!(
                    "{} Webhook failed ({}): {}",
                    "[WEBHOOK ERROR]".red(),
                    status,
                    body
                );
            }
        }
        Err(e) => {
            eprintln!("{} Failed to send webhook: {}", "[WEBHOOK ERROR]".red(), e);
        }
    }
}

fn build_discord_embed(a: &ExecutionAlert) -> serde_json::Value {
    let side_upper = a.side.to_uppercase();
    let color = if side_upper == "YES" { 0x00cc66 } else { 0xff4444 };

    json!({
        "embeds": [{
            "title": format!("Trade Executed: {} {}", side_upper, a.kalshi_ticker),
            "description": format!("Matched from Polymarket: *{}*", escape_special_chars(&a.poly_title)),
            "color": color,
            "fields": [
                { "name": "Side",     "value": side_upper,                                          "inline": true },
                { "name": "Qty",      "value": format!("{}", a.count),                              "inline": true },
                { "name": "Price",    "value": format!("{}c", a.price_cents),                       "inline": true },
                { "name": "Fee",      "value": format!("{}c/contract", a.fee_cents),                "inline": true },
                { "name": "Cost",     "value": format!("${:.2}", a.total_cost_cents as f64 / 100.0),"inline": true },
                { "name": "EV",       "value": format!("+{:.1}c/contract", a.ev_cents),             "inline": true },
                { "name": "Kelly",    "value": format!("{:.2}%", a.kelly_pct),                      "inline": true },
                { "name": "Win Rate", "value": format!("{:.1}%", a.whale_win_rate * 100.0),         "inline": true },
                { "name": "Balance",  "value": format!("${:.2}", a.balance_after_cents as f64 / 100.0), "inline": true },
            ],
            "footer": { "text": format!("Order: {}", a.order_id) },
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }]
    })
}

fn build_generic_payload(a: &ExecutionAlert) -> serde_json::Value {
    json!({
        "event": "trade_executed",
        "kalshi_ticker": a.kalshi_ticker,
        "side": a.side,
        "count": a.count,
        "price_cents": a.price_cents,
        "fee_cents": a.fee_cents,
        "total_cost_cents": a.total_cost_cents,
        "ev_cents": a.ev_cents,
        "kelly_pct": a.kelly_pct,
        "whale_win_rate": a.whale_win_rate,
        "balance_after_cents": a.balance_after_cents,
        "poly_title": a.poly_title,
        "order_id": a.order_id,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })
}
