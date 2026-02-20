use colored::*;

use crate::alerts::sound;
use crate::alerts::webhook;

pub async fn test_sound() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "TESTING ALERT SOUND".bright_cyan().bold());
    println!();
    println!("Playing single alert...");
    sound::play_alert_sound();

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    println!("Playing triple alert (for repeat actors)...");
    sound::play_triple_beep();

    println!();
    println!("{}", "Sound test complete.".bright_green());
    println!("If you didn't hear anything, check:");
    println!("  1. System volume is not muted");
    println!("  2. Sound file exists: /System/Library/Sounds/Ping.aiff");
    println!("  3. Try: afplay /System/Library/Sounds/Ping.aiff");

    Ok(())
}

pub async fn test_webhook(_conn: &rusqlite::Connection) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "TESTING WEBHOOK".bright_cyan().bold());
    println!();

    let config = match crate::config::load_config() {
        Ok(cfg) => cfg,
        Err(_) => {
            println!(
                "{}",
                "No configuration found. Run 'wwatcher setup' first.".red()
            );
            return Ok(());
        }
    };

    let webhook_url = config.webhook_url.or(config.discord_webhook_url);
    let webhook_url = match webhook_url {
        Some(url) => url,
        None => {
            println!(
                "{}",
                "No webhook configured. Run 'wwatcher setup' to add a webhook URL.".red()
            );
            return Ok(());
        }
    };

    println!("Sending test execution alert to: {}", webhook_url.bright_green());
    println!();

    let test_alert = webhook::ExecutionAlert {
        kalshi_ticker: "KXNBAGAME-26FEB20-TEST".to_string(),
        side: "yes".to_string(),
        count: 5,
        price_cents: 62,
        fee_cents: 2,
        total_cost_cents: 320,
        ev_cents: 23.0,
        kelly_pct: 1.85,
        whale_win_rate: 0.87,
        balance_after_cents: 4680,
        poly_title: "Lakers vs Celtics: Will the Lakers win?".to_string(),
        order_id: "test-order-12345".to_string(),
    };

    webhook::send_execution_alert(&webhook_url, &test_alert).await;

    println!("{}", "Test execution alert sent!".bright_green());
    println!();
    println!("You should see a Discord embed (or JSON payload) with:");
    println!("  Ticker:   KXNBAGAME-26FEB20-TEST");
    println!("  Side:     YES @ 62c");
    println!("  Qty:      5 contracts");
    println!("  Cost:     $3.20");
    println!("  EV:       +23.0c/contract");
    println!("  Win Rate: 87.0%");

    Ok(())
}

pub async fn test_profile(wallet_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", format!("TESTING WHALE PROFILE: {}", wallet_id).bright_cyan().bold());
    println!();

    let mut cache = crate::whale_profile::WhaleProfileCache::new();
    
    println!("Fetching live profile from Polymarket Data API...");
    match crate::whale_profile::fetch_whale_profile(wallet_id, &mut cache).await {
        Some(profile) => {
            println!("{}", "✅ Profile Fetched Successfully".bright_green());
            println!("{:#?}", profile);
            
            if let Some(wr) = profile.win_rate {
                println!("Win Rate: {:.1}%", wr * 100.0);
            } else {
                println!("Win Rate: N/A");
            }
        }
        None => {
            println!("{}", "❌ Failed to fetch profile".bright_red());
        }
    }

    Ok(())
}
