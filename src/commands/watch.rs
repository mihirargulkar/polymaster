use std::sync::OnceLock;
use std::time::Duration;

use colored::*;
use rusqlite::Connection;
use tokio::time;

use crate::alerts::AlertData;
use crate::alerts::display::{self, format_number, print_kalshi_alert, print_market_context, print_order_book, print_top_holders, print_whale_alert, print_whale_profile};
use crate::alerts::history;
use crate::categories::CategoryRegistry;
use crate::db;
use crate::platforms::kalshi;
use crate::platforms::polymarket;
use crate::types;
use crate::whale_profile;
use crate::execution::matcher::MarketMatcher;
use crate::execution::kalshi::KalshiExecutor;

fn resolve_pem(input: &str) -> String {
    if input.starts_with('/') || input.starts_with('.') || input.contains('/') {
        std::fs::read_to_string(input).unwrap_or_else(|_| input.to_string())
    } else {
        input.to_string()
    }
}

fn shared_http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(4)
            .build()
            .expect("failed to build shared HTTP client")
    })
}

// ── Kalshi fee + risk math ───────────────────────────────────────────

/// Kalshi taker fee per contract in cents: ceil(7 × P × (100-P) / 10000), capped at 2c.
fn kalshi_taker_fee_cents(price_cents: i64) -> i64 {
    let p = price_cents;
    let q = 100 - price_cents;
    let raw = 7 * p * q; // scaled by 10000
    let fee = (raw + 9999) / 10000; // ceiling division
    fee.min(2).max(0)
}

/// Expected value per contract in cents, after fees.
/// EV = win_rate × (100 - price) - (1 - win_rate) × price - fee
///    = 100 × win_rate - price - fee
fn expected_value_cents(win_rate: f64, price_cents: i64, fee_cents: i64) -> f64 {
    100.0 * win_rate - price_cents as f64 - fee_cents as f64
}

/// Quarter-Kelly bet fraction: (EV / payout_if_win) / 4, clamped to [0, max_frac].
fn quarter_kelly_fraction(win_rate: f64, price_cents: i64, fee_cents: i64, max_frac: f64) -> f64 {
    let ev = expected_value_cents(win_rate, price_cents, fee_cents);
    if ev <= 0.0 {
        return 0.0;
    }
    let payout = (100 - price_cents) as f64;
    if payout <= 0.0 {
        return 0.0;
    }
    let full_kelly = ev / payout;
    (full_kelly / 4.0).min(max_frac).max(0.0)
}

async fn closes_within_24h(ticker: &str) -> bool {
    let url = format!(
        "https://api.elections.kalshi.com/trade-api/v2/markets/{}",
        ticker
    );
    let resp = match shared_http_client()
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => r,
        _ => return false,
    };

    let text = match resp.text().await {
        Ok(t) => t,
        Err(_) => return false,
    };
    let parsed: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let market = match parsed.get("market") {
        Some(m) => m,
        None => return false,
    };

    let expiry_str = market
        .get("expected_expiration_time")
        .or_else(|| market.get("close_time"))
        .and_then(|v| v.as_str());

    let expiry_str = match expiry_str {
        Some(s) => s,
        None => return false,
    };

    match chrono::DateTime::parse_from_rfc3339(expiry_str) {
        Ok(expiry) => {
            let expiry_utc = expiry.with_timezone(&chrono::Utc);
            let hours_left = (expiry_utc - chrono::Utc::now()).num_hours();
            println!("⏰ Market {} closes in ~{}h", ticker, hours_left);
            hours_left <= 24 && hours_left >= 0
        }
        Err(_) => false,
    }
}

pub async fn watch_whales(threshold: u64, interval: u64, conn: Connection) -> Result<(), Box<dyn std::error::Error>> {
    // Display disclaimer
    println!("{}", "=".repeat(70).bright_yellow());
    println!("{}", "DISCLAIMER".bright_yellow().bold());
    println!("This tool is for informational and research purposes only.");
    println!("I do not condone gambling or speculative trading.");
    println!("Use this data solely for informed decision-making and market analysis.");
    println!("Trade responsibly and within your means.");
    println!("{}", "=".repeat(70).bright_yellow());
    println!();

    let config = crate::config::load_config().ok();
    let mut prune_counter = 0;

    println!("\n{} ...", "WHALE WATCHER ACTIVE".bright_green().bold());
    println!(
        "Threshold: {}",
        format!("${}", format_number(threshold)).bright_green()
    );
    println!("Interval:  {} seconds", interval);

    // Initialize category filtering
    let category_registry = CategoryRegistry::new();
    let selected_categories: Vec<String> = config
        .as_ref()
        .map(|c| c.categories.clone())
        .unwrap_or_else(|| vec!["all".into()]);

    if selected_categories.iter().any(|s| s == "all") {
        println!("Categories: {}", "All markets".bright_green());
    } else {
        println!(
            "Categories: {}",
            selected_categories.join(", ").bright_green()
        );
    }

    // Platform filtering
    let selected_platforms: Vec<String> = config
        .as_ref()
        .map(|c| c.platforms.clone())
        .unwrap_or_else(|| vec!["all".into()]);
    let watch_polymarket = selected_platforms.iter().any(|p| p == "all" || p == "polymarket");
    let watch_kalshi = selected_platforms.iter().any(|p| p == "all" || p == "kalshi");

    if watch_polymarket && watch_kalshi {
        println!("Platforms:  {}", "Polymarket + Kalshi".bright_green());
    } else if watch_polymarket {
        println!("Platforms:  {}", "Polymarket only".bright_green());
    } else {
        println!("Platforms:  {}", "Kalshi only".bright_green());
    }

    if let Some(ref cfg) = config {
        if cfg.webhook_url.is_some() {
            println!("Webhook:   {}", "Enabled".bright_green());
        }
    }

    // Show DB info
    let alert_count = db::alert_count(&conn);
    println!("Database:  {} alerts stored", alert_count.to_string().bright_white());
    println!();

    let mut last_polymarket_trade_id: Option<String> = None;
    let mut last_kalshi_trade_id: Option<String> = None;
    let mut first_poll_poly = true;
    let mut first_poll_kalshi = true;

    let mut wallet_tracker = types::WalletTracker::new();
    let mut whale_cache = whale_profile::WhaleProfileCache::new();

    // ── Risk management state ───────────────────────────────────────────
    let max_open = config.as_ref().map(|c| c.max_open_positions).unwrap_or(5);
    let daily_loss_frac = config.as_ref().map(|c| c.daily_loss_limit).unwrap_or(0.10);
    let reserve_frac = config.as_ref().map(|c| c.reserve_fraction).unwrap_or(0.20);
    let max_bet_frac = config.as_ref().map(|c| c.max_bet_fraction).unwrap_or(0.02);
    let max_bet_cap = config.as_ref().map(|c| c.max_bet_cap).unwrap_or(10.0);
    let max_entry_cents: i64 = config.as_ref().map(|c| c.max_entry_price_cents).unwrap_or(97);
    let mut day_start_balance_cents: Option<i64> = None;
    let mut daily_loss_cents: i64 = 0;

    // Initialize Execution Modules (Ollama for Polymarket→Kalshi matching)
    let (ollama_model, ollama_embed_model, ollama_url) = config
        .as_ref()
        .map(|c| (c.ollama_model.clone(), c.ollama_embed_model.clone(), c.ollama_url.clone()))
        .unwrap_or_else(|| ("llama3".into(), "nomic-embed-text".into(), "http://localhost:11434".into()));
    let mut matcher = MarketMatcher::new(ollama_model, ollama_embed_model, Some(&ollama_url));
    let mut executed_tickers: std::collections::HashSet<String> = std::collections::HashSet::new();
    let kalshi_executor = if let Some(ref cfg) = config {
        if let (Some(key_id), Some(private_key_input)) = (&cfg.kalshi_api_key_id, &cfg.kalshi_private_key) {
             let private_key_pem = resolve_pem(private_key_input);

             match KalshiExecutor::new(key_id.clone(), &private_key_pem, cfg.kalshi_is_demo) {
                 Ok(ex) => {
                     println!("Execution: {}", "Kalshi Executor Ready".bright_green());
                     Some(ex)
                 },
                 Err(e) => {
                     eprintln!("Execution Init Failed: {}", e);
                     None
                 }
             }
        } else { None }
    } else { None };

    // Seed executed_tickers with existing open Kalshi positions so we don't double-up
    if let Some(ref executor) = kalshi_executor {
        match executor.get_open_event_tickers().await {
            Ok(open_events) => {
                if !open_events.is_empty() {
                    println!("📋 Loaded {} existing Kalshi positions into dedup set:", open_events.len());
                    for et in &open_events {
                        println!("   • {}", et);
                    }
                    executed_tickers.extend(open_events);
                } else {
                    println!("📋 No existing Kalshi positions — dedup set empty");
                }
            }
            Err(e) => eprintln!("⚠️ Could not load existing positions: {}", e),
        }
    }

    // Start Kalshi WebSocket if watching Kalshi
    let mut kalshi_ws_rx = if watch_kalshi {
        println!("Kalshi WS:  {}", "Connecting...".bright_cyan());
        let (api_id, priv_key_raw) = config.as_ref().map(|c| (c.kalshi_api_key_id.clone(), c.kalshi_private_key.clone())).unwrap_or((None, None));
        let priv_key = priv_key_raw.map(|k| resolve_pem(&k));
        Some(crate::ws::kalshi::spawn_kalshi_ws(api_id, priv_key))
    } else {
        None
    };
    // Track whether WS is producing trades (for fallback)
    let mut kalshi_ws_last_trade = std::time::Instant::now();
    let kalshi_ws_fallback_threshold = Duration::from_secs(interval * 12); // fall back to HTTP if no WS trades in ~1 min

    let mut tick_interval = time::interval(Duration::from_secs(interval));

    let mut kalshi_market_cache: std::collections::HashMap<String, Option<kalshi::MarketInfo>> = std::collections::HashMap::new();
    let mut kalshi_context_cache: std::collections::HashMap<String, Option<crate::alerts::MarketContext>> = std::collections::HashMap::new();

    loop {
        tick_interval.tick().await;

        // Periodic cleanup and cache refresh
        prune_counter += 1;
        if prune_counter >= 60 {
            prune_counter = 0;

            matcher.prune_cache();
            db::prune_wallet_memory(&conn);
            let retention = config.as_ref().map(|c| c.history_retention_days).unwrap_or(30);
            db::prune_old_alerts(&conn, retention);
            whale_cache.prune();
            kalshi_market_cache.clear();
            kalshi_context_cache.clear();
        }
        wallet_tracker.maybe_refresh_cache(&conn);

        // Drain Kalshi WebSocket trades (non-blocking)
        if let Some(ref mut rx) = kalshi_ws_rx {
            while let Ok(ws_trade) = rx.try_recv() {
                kalshi_ws_last_trade = std::time::Instant::now();

                let taker_price = if ws_trade.taker_side.eq_ignore_ascii_case("no") {
                    ws_trade.no_price
                } else {
                    ws_trade.yes_price
                };
                let trade_value = (taker_price / 100.0) * f64::from(ws_trade.count);
                if trade_value < threshold as f64 {
                    continue;
                }

                let mut trade = kalshi::Trade {
                    trade_id: ws_trade.trade_id.clone(),
                    ticker: ws_trade.ticker.clone(),
                    price: taker_price / 100.0,
                    count: ws_trade.count,
                    yes_price: ws_trade.yes_price,
                    no_price: ws_trade.no_price,
                    taker_side: ws_trade.taker_side.clone(),
                    created_time: ws_trade.created_time.clone(),
                    market_title: None,
                };

                let ticker = trade.ticker.clone();
                let market_info = if let Some(info) = kalshi_market_cache.get(&ticker) {
                    info.clone()
                } else {
                    let info = match tokio::time::timeout(Duration::from_secs(2), kalshi::fetch_market_info_full(&ticker)).await {
                        Ok(res) => res,
                        Err(_) => None,
                    };
                    kalshi_market_cache.insert(ticker.clone(), info.clone());
                    info
                };

                if let Some(ref info) = market_info {
                    trade.market_title = Some(info.title.clone());
                }

                // Category filter
                if let Some(ref title) = trade.market_title {
                    let has_native_match = market_info.as_ref()
                        .and_then(|info| info.category.as_ref())
                        .map(|cat| category_registry.matches_native_category(cat, &selected_categories))
                        .unwrap_or(false);

                    if !has_native_match {
                        if category_registry
                            .matches_selection(title, &selected_categories)
                            .is_none()
                        {
                            continue;
                        }
                    }
                }

                let outcome = kalshi::parse_ticker_details(&trade.ticker, &trade.taker_side);
                let action = trade.taker_side.to_uppercase();

                // Fetch market context early for filtering (with cache and timeout)
                let market_ctx = if let Some(ctx) = kalshi_context_cache.get(&ticker) {
                    ctx.clone()
                } else {
                    let ctx = match tokio::time::timeout(Duration::from_secs(2), kalshi::fetch_market_context(&ticker)).await {
                        Ok(res) => res,
                        Err(_) => None,
                    };
                    kalshi_context_cache.insert(ticker.clone(), ctx.clone());
                    ctx
                };

                // Odds and spread filter
                if let Some(ref cfg) = config {
                    if let Some(ref ctx) = market_ctx {
                        // Skip if odds too high (near-certainty)
                        if ctx.yes_price > cfg.max_odds || ctx.no_price > cfg.max_odds {
                            continue;
                        }
                        // Skip if spread too low (dead market)
                        if cfg.min_spread > 0.0 && ctx.spread < cfg.min_spread {
                            continue;
                        }
                    }
                }

                print_kalshi_alert(&trade, trade_value, None);

                if let Some(ref ctx) = market_ctx {
                    print_market_context(ctx);
                }

                let order_book = match tokio::time::timeout(Duration::from_secs(2), kalshi::fetch_order_book(&ticker)).await {
                    Ok(res) => res,
                    Err(_) => None,
                };
                if let Some(ref ob) = order_book {
                    print_order_book(ob);
                }

                let alert_data = AlertData {
                    platform: "Kalshi",
                    market_title: trade.market_title.as_deref(),
                    market_id: Some(&trade.ticker),
                    outcome: Some(&outcome),
                    side: &action,
                    value: trade_value,
                    price: trade.yes_price / 100.0,
                    size: f64::from(trade.count),
                    timestamp: &trade.created_time,
                    wallet_id: None,
                    wallet_activity: None,
                    market_context: market_ctx.as_ref(),
                    whale_profile: None,
                    order_book: order_book.as_ref(),
                    top_holders: None,
                };

                history::log_alert(&alert_data, &conn);
            }
        }

        // Determine if we should use HTTP polling for Kalshi (fallback if WS is silent)
        let kalshi_ws_active = kalshi_ws_rx.is_some()
            && kalshi_ws_last_trade.elapsed() < kalshi_ws_fallback_threshold;

        // Check Polymarket
        if watch_polymarket { match polymarket::fetch_recent_trades(Some(threshold)).await {
            Ok(mut trades) => {
                if let Some(first_trade) = trades.first() {
                    let new_last_id = first_trade.id.clone();

                    if first_poll_poly {
                        first_poll_poly = false;
                        last_polymarket_trade_id = Some(new_last_id.clone());
                        println!("📌 Polymarket bookmark set — only new trades from now on");
                        trades.clear();
                    }

                    for trade in &mut trades {
                        if let Some(ref last_id) = last_polymarket_trade_id {
                            if trade.id == *last_id {
                                break;
                            }
                        }

                        let trade_value = trade.size * trade.price;
                        if trade_value >= threshold as f64 {
                            // Category filter: skip if market doesn't match selected categories
                            if let Some(ref title) = trade.market_title {
                                if category_registry
                                    .matches_selection(title, &selected_categories)
                                    .is_none()
                                {
                                    continue;
                                }
                            }

                            let wallet_activity = if let Some(ref wallet_id) = trade.wallet_id {
                                wallet_tracker.record_transaction(wallet_id, trade_value);
                                Some(wallet_tracker.get_activity(wallet_id))
                            } else {
                                None
                            };

                            // Check for returning whale (12h memory)
                            let whale_scenario = trade.wallet_id.as_deref().and_then(|wid| {
                                wallet_tracker.classify_whale_return(
                                    &conn,
                                    wid,
                                    Some(&trade.asset_id),
                                    trade.outcome.as_deref(),
                                )
                            });

                            // Fetch market context early for filtering
                            let market_ctx = polymarket::fetch_market_context(&trade.market).await;

                            // Odds and spread filter
                            if let Some(ref cfg) = config {
                                if let Some(ref ctx) = market_ctx {
                                    // Skip if odds too high (near-certainty)
                                    if ctx.yes_price > cfg.max_odds || ctx.no_price > cfg.max_odds {
                                        continue;
                                    }
                                    // Skip if spread too low (dead market)
                                    if cfg.min_spread > 0.0 && ctx.spread < cfg.min_spread {
                                        continue;
                                    }
                                }
                            }

                            // Print returning whale info if detected
                            if let Some(ref scenario) = whale_scenario {
                                display::print_returning_whale(scenario, "Polymarket");
                            }

                            print_whale_alert(
                                "Polymarket",
                                trade,
                                trade_value,
                                wallet_activity.as_ref(),
                            );

                            if let Some(ref ctx) = market_ctx {
                                print_market_context(ctx);
                            }

                            // Fetch whale profile (Polymarket only - on-chain wallets)
                            let wp = if let Some(ref wallet_id) = trade.wallet_id {
                                whale_profile::fetch_whale_profile(wallet_id, &mut whale_cache).await
                            } else {
                                None
                            };
                            if let Some(ref profile) = wp {
                                print_whale_profile(profile);
                            }

                            // Fetch order book depth
                            let order_book = polymarket::fetch_order_book(&trade.asset_id).await;
                            if let Some(ref ob) = order_book {
                                print_order_book(ob);
                            }

                            // Fetch top holders
                            let top_holders = polymarket::fetch_top_holders(&trade.market).await;
                            if let Some(ref th) = top_holders {
                                print_top_holders(th);
                            }

                            let alert_data = AlertData {
                                platform: "Polymarket",
                                market_title: trade.market_title.as_deref(),
                                market_id: Some(&trade.market),
                                outcome: trade.outcome.as_deref(),
                                side: &trade.side,
                                value: trade_value,
                                price: trade.price,
                                size: trade.size,
                                timestamp: &trade.timestamp,
                                wallet_id: trade.wallet_id.as_deref(),
                                wallet_activity: wallet_activity.as_ref(),
                                market_context: market_ctx.as_ref(),
                                whale_profile: wp.as_ref(),
                                order_book: order_book.as_ref(),
                                top_holders: top_holders.as_ref(),
                            };

                            let alert_id = history::log_alert(&alert_data, &conn);

                            // ═══ RISK-MANAGED EXECUTION PIPELINE ═══════════════
                            let whale_win_rate = wp.as_ref().and_then(|p| p.win_rate);

                            // Gate 1: Win rate
                            let passes_win_rate = match whale_win_rate {
                                Some(wr) if wr >= 0.85 => {
                                    println!("✅ Whale win rate {:.1}% passes 85% threshold", wr * 100.0);
                                    true
                                }
                                Some(wr) => {
                                    println!("⚠️ Skipping execution: whale win rate {:.1}% < 85%", wr * 100.0);
                                    false
                                }
                                None => {
                                    println!("⚠️ Skipping execution: whale win rate unknown");
                                    false
                                }
                            };

                            let poly_title = trade.market_title.as_deref().unwrap_or("");
                            if passes_win_rate && !poly_title.is_empty() {
                                let search_results = kalshi::search_markets(poly_title).await.unwrap_or_default();
                                if let Some(match_result) = matcher.match_market(
                                    poly_title,
                                    trade.outcome.as_deref().unwrap_or(""),
                                    &search_results
                                ).await {
                                    println!("{} Matched to Kalshi: {} ({}) Confidence: {:.2}",
                                        "🤖 LLM".bright_magenta(),
                                        match_result.ticker.bright_cyan(),
                                        match_result.side,
                                        match_result.confidence.unwrap_or(0.0)
                                    );

                                    let dedup_key = match match_result.ticker.rfind('-') {
                                        Some(pos) => match_result.ticker[..pos].to_string(),
                                        None => match_result.ticker.clone(),
                                    };

                                    // Gate 2: Event-level dedup
                                    if executed_tickers.contains(&dedup_key) {
                                        println!("⚠️ Already have position on event {} — skipping",
                                            dedup_key);
                                    }
                                    // Gate 3: Max open positions
                                    else if executed_tickers.len() >= max_open {
                                        println!("⚠️ Max {} open positions reached — skipping {}",
                                            max_open, match_result.ticker);
                                    }
                                    // Gate 4: 24h expiry
                                    else if !closes_within_24h(&match_result.ticker).await {
                                        println!("⚠️ Skipping {}: does not close within 24 hours",
                                            match_result.ticker);
                                    }
                                    else if let Some(ref executor) = kalshi_executor {
                                        // Gate 5: Live Kalshi position check
                                        if executor.has_open_position(&dedup_key).await.unwrap_or(false) {
                                            println!("⚠️ Already have LIVE Kalshi position on {} — skipping",
                                                dedup_key);
                                            executed_tickers.insert(dedup_key.clone());
                                        } else {

                                        // ── Fee + EV calculation ────────────────────────────
                                        let whale_price_cents = (trade.price * 100.0).round() as i64;
                                        let price_cents = whale_price_cents.clamp(1, 99);
                                        let fee_cents = kalshi_taker_fee_cents(price_cents);
                                        let wr = whale_win_rate.unwrap_or(0.0);
                                        let ev_cents = expected_value_cents(wr, price_cents, fee_cents);

                                        println!("📊 Price: {}c | Fee: {}c/contract | EV: {:.1}c/contract (WR {:.1}%)",
                                            price_cents, fee_cents, ev_cents, wr * 100.0);

                                        // Gate 6: Max entry price
                                        if price_cents > max_entry_cents {
                                            println!("⚠️ Skipping: price {}c > max {}c",
                                                price_cents, max_entry_cents);
                                        }
                                        // Gate 7: Positive expected value after fees
                                        else if ev_cents <= 0.0 {
                                            println!("⚠️ Skipping: negative EV {:.1}c after {}c fee (need WR > {:.0}%)",
                                                ev_cents, fee_cents, (price_cents + fee_cents) as f64);
                                        } else {

                                        // ── Balance + risk sizing ───────────────────────────
                                        let balance_cents = executor.get_balance().await.unwrap_or(0);

                                        if day_start_balance_cents.is_none() {
                                            day_start_balance_cents = Some(balance_cents);
                                            println!("📋 Day-start balance: ${:.2}", balance_cents as f64 / 100.0);
                                        }
                                        let day_start = day_start_balance_cents.unwrap_or(balance_cents);

                                        // Gate 8: Daily loss limit
                                        let loss_limit_cents = (day_start as f64 * daily_loss_frac) as i64;
                                        if daily_loss_cents >= loss_limit_cents {
                                            println!("🛑 Daily loss limit hit: lost ${:.2} >= ${:.2} limit — halting trades",
                                                daily_loss_cents as f64 / 100.0,
                                                loss_limit_cents as f64 / 100.0);
                                        }
                                        // Gate 9: Reserve
                                        else {
                                        let reserve_cents = (day_start as f64 * reserve_frac) as i64;

                                        // ── Quarter-Kelly sizing ────────────────────────────
                                        let kelly_frac = quarter_kelly_fraction(wr, price_cents, fee_cents, max_bet_frac);
                                        let kelly_dollars = (balance_cents as f64 / 100.0) * kelly_frac;
                                        let bet_size = kelly_dollars
                                            .min(max_bet_cap)
                                            .max(1.0); // $1 floor
                                        let count = ((bet_size * 100.0) / price_cents as f64).max(1.0) as i32;
                                        let trade_cost_cents = (count as i64) * price_cents;
                                        let total_cost_with_fees = trade_cost_cents + (count as i64) * fee_cents;

                                        println!("📐 Kelly: {:.2}% → ${:.2} | {} contracts @ {}c + {}c fee = ${:.2}",
                                            kelly_frac * 100.0,
                                            bet_size,
                                            count,
                                            price_cents,
                                            fee_cents,
                                            total_cost_with_fees as f64 / 100.0);

                                        if balance_cents.saturating_sub(total_cost_with_fees) < reserve_cents {
                                            println!("⚠️ Skipping: ${:.2} - ${:.2} would breach {:.0}% reserve (${:.2})",
                                                balance_cents as f64 / 100.0,
                                                total_cost_with_fees as f64 / 100.0,
                                                reserve_frac * 100.0,
                                                reserve_cents as f64 / 100.0);
                                        } else {
                                            println!("💰 Balance: ${:.2} → cost ${:.2} → ${:.2} remaining",
                                                balance_cents as f64 / 100.0,
                                                total_cost_with_fees as f64 / 100.0,
                                                (balance_cents - total_cost_with_fees) as f64 / 100.0);

                                            println!("🚀 EXECUTING: Buy {} {} @ {}c (Qty: {}, ${:.2}, EV: +{:.1}c/contract)",
                                                match_result.side.to_uppercase(),
                                                match_result.ticker,
                                                price_cents,
                                                count,
                                                count as f64 * price_cents as f64 / 100.0,
                                                ev_cents
                                            );

                                            match executor.place_order(
                                                &match_result.ticker,
                                                &match_result.side,
                                                count,
                                                price_cents
                                            ).await {
                                                Ok(order_id) => {
                                                    println!("✅ Order Placed: {}", order_id.to_string().green());
                                                    executed_tickers.insert(dedup_key.clone());
                                                    daily_loss_cents += trade_cost_cents;

                                                    if let Some(row_id) = alert_id {
                                                        db::mark_alert_executed(
                                                            &conn,
                                                            row_id,
                                                            &order_id.to_string(),
                                                            &match_result.ticker,
                                                            &match_result.side,
                                                            bet_size,
                                                            price_cents as f64 / 100.0
                                                        );
                                                    }

                                                    let balance_after = balance_cents.saturating_sub(total_cost_with_fees);
                                                    if let Some(ref cfg) = config {
                                                        let url = cfg.webhook_url.as_ref()
                                                            .or(cfg.discord_webhook_url.as_ref());
                                                        if let Some(url) = url {
                                                            let exec_alert = crate::alerts::webhook::ExecutionAlert {
                                                                kalshi_ticker: match_result.ticker.clone(),
                                                                side: match_result.side.clone(),
                                                                count,
                                                                price_cents,
                                                                fee_cents,
                                                                total_cost_cents: total_cost_with_fees,
                                                                ev_cents,
                                                                kelly_pct: kelly_frac * 100.0,
                                                                whale_win_rate: wr,
                                                                balance_after_cents: balance_after,
                                                                poly_title: poly_title.to_string(),
                                                                order_id: order_id.to_string(),
                                                            };
                                                            println!("📨 Sending execution alert...");
                                                            crate::alerts::webhook::send_execution_alert(url, &exec_alert).await;
                                                        }
                                                    }
                                                },
                                                Err(e) => eprintln!("❌ Execution Failed: {}", e),
                                            }
                                        }
                                        }
                                        }
                                        }
                                    } else {
                                        println!("⚠️ Execution skipped (No credentials)");
                                    }
                                }
                            }
                            // ═══ END EXECUTION PIPELINE ════════════════════════

                            // Record to wallet memory DB
                            if let Some(ref wallet_id) = trade.wallet_id {
                                wallet_tracker.record_to_db(
                                    &conn,
                                    wallet_id,
                                    trade.market_title.as_deref(),
                                    Some(&trade.asset_id),
                                    trade.outcome.as_deref(),
                                    &trade.side,
                                    trade_value,
                                    trade.price,
                                    "Polymarket",
                                );
                            }
                        }
                    }

                    last_polymarket_trade_id = Some(new_last_id);
                }
            }
            Err(e) => {
                eprintln!("{} {}", "[ERROR] Polymarket:".red(), e);
            }
        } } // end if watch_polymarket

        // Check Kalshi (HTTP polling fallback — only when WebSocket isn't active)
        if watch_kalshi && !kalshi_ws_active { match kalshi::fetch_recent_trades(config.as_ref()).await {
            Ok(mut trades) => {
                if let Some(first_trade) = trades.first() {
                    let new_last_id = first_trade.trade_id.clone();

                    if first_poll_kalshi {
                        first_poll_kalshi = false;
                        last_kalshi_trade_id = Some(new_last_id.clone());
                        println!("📌 Kalshi bookmark set — only new trades from now on");
                        trades.clear();
                    }

                    for trade in &mut trades {
                        if let Some(ref last_id) = last_kalshi_trade_id {
                            if trade.trade_id == *last_id {
                                break;
                            }
                        }

                        let taker_price_http = if trade.taker_side.eq_ignore_ascii_case("no") {
                            trade.no_price
                        } else {
                            trade.yes_price
                        };
                        let trade_value = (taker_price_http / 100.0) * f64::from(trade.count);
                        if trade_value >= threshold as f64 {
                            let market_info = kalshi::fetch_market_info_full(&trade.ticker).await;
                            if let Some(ref info) = market_info {
                                trade.market_title = Some(info.title.clone());
                            }

                            // Category filter: use native Kalshi category when available,
                            // fall back to keyword matching on title
                            if let Some(ref title) = trade.market_title {
                                let has_native_match = market_info.as_ref()
                                    .and_then(|info| info.category.as_ref())
                                    .map(|cat| category_registry.matches_native_category(cat, &selected_categories))
                                    .unwrap_or(false);

                                if !has_native_match {
                                    if category_registry
                                        .matches_selection(title, &selected_categories)
                                        .is_none()
                                    {
                                        continue;
                                    }
                                }
                            }

                            let outcome =
                                kalshi::parse_ticker_details(&trade.ticker, &trade.taker_side);

                            let action = trade.taker_side.to_uppercase();

                            // Fetch market context early for filtering
                            let market_ctx = kalshi::fetch_market_context(&trade.ticker).await;

                            // Odds and spread filter
                            if let Some(ref cfg) = config {
                                if let Some(ref ctx) = market_ctx {
                                    // Skip if odds too high (near-certainty)
                                    if ctx.yes_price > cfg.max_odds || ctx.no_price > cfg.max_odds {
                                        continue;
                                    }
                                    // Skip if spread too low (dead market)
                                    if cfg.min_spread > 0.0 && ctx.spread < cfg.min_spread {
                                        continue;
                                    }
                                }
                            }

                            print_kalshi_alert(trade, trade_value, None);

                            if let Some(ref ctx) = market_ctx {
                                print_market_context(ctx);
                            }

                            // Fetch order book depth for Kalshi
                            let order_book = kalshi::fetch_order_book(&trade.ticker).await;
                            if let Some(ref ob) = order_book {
                                print_order_book(ob);
                            }

                            let alert_data = AlertData {
                                platform: "Kalshi",
                                market_title: trade.market_title.as_deref(),
                                market_id: Some(&trade.ticker),
                                outcome: Some(&outcome),
                                side: &action,
                                value: trade_value,
                                price: trade.yes_price / 100.0,
                                size: f64::from(trade.count),
                                timestamp: &trade.created_time,
                                wallet_id: None,
                                wallet_activity: None,
                                market_context: market_ctx.as_ref(),
                                whale_profile: None,
                                order_book: order_book.as_ref(),
                                top_holders: None,
                            };

                            history::log_alert(&alert_data, &conn);
                        }
                    }

                    last_kalshi_trade_id = Some(new_last_id);
                }
            }
            Err(e) => {
                eprintln!("{} {}", "[ERROR] Kalshi:".red(), e);
            }
        } } // end if watch_kalshi
    }
}
