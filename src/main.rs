mod config;
mod kalshi;
mod polymarket;
mod types;

use clap::{Parser, Subcommand};
use colored::*;
use std::time::Duration;
use tokio::time;

#[derive(Parser)]
#[command(name = "whale-watcher")]
#[command(about = "Monitor large transactions on Polymarket and Kalshi", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Watch for large transactions (default threshold: $25,000)
    Watch {
        /// Minimum transaction size to alert on (in USD)
        #[arg(short, long, default_value = "25000")]
        threshold: u64,

        /// Polling interval in seconds
        #[arg(short, long, default_value = "5")]
        interval: u64,
    },
    /// Configure API credentials
    Setup,
    /// Show current configuration
    Status,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Setup => {
            setup_config().await?;
        }
        Commands::Status => {
            show_status().await?;
        }
        Commands::Watch {
            threshold,
            interval,
        } => {
            watch_whales(threshold, interval).await?;
        }
    }

    Ok(())
}

async fn setup_config() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "WHALE WATCHER SETUP".bright_cyan().bold());
    println!();

    println!("This tool monitors large transactions on Polymarket and Kalshi.");
    println!("API credentials are optional - the tool works with public data.");
    println!();

    // Get Kalshi credentials (optional)
    println!("{}", "Kalshi Configuration (optional):".bright_yellow());
    println!("Generate API keys at: https://kalshi.com/profile/api-keys");
    
    print!("Enter Kalshi API Key ID (or press Enter to skip): ");
    let mut kalshi_key_id = String::new();
    std::io::stdin().read_line(&mut kalshi_key_id)?;
    let kalshi_key_id = kalshi_key_id.trim().to_string();

    let kalshi_private_key = if !kalshi_key_id.is_empty() {
        print!("Enter Kalshi Private Key: ");
        let mut key = String::new();
        std::io::stdin().read_line(&mut key)?;
        key.trim().to_string()
    } else {
        String::new()
    };

    println!();
    println!("{}", "Polymarket Configuration:".bright_yellow());
    println!("Polymarket data is publicly accessible - no API key needed!");
    println!();

    // Save configuration
    let config = config::Config {
        kalshi_api_key_id: if kalshi_key_id.is_empty() {
            None
        } else {
            Some(kalshi_key_id)
        },
        kalshi_private_key: if kalshi_private_key.is_empty() {
            None
        } else {
            Some(kalshi_private_key)
        },
    };

    config::save_config(&config)?;

    println!("{}", "Configuration saved successfully.".bright_green());
    println!();
    println!("Run {} to start watching for whale transactions.", "whale-watcher watch".bright_cyan());

    Ok(())
}

async fn show_status() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "WHALE WATCHER STATUS".bright_cyan().bold());
    println!();

    match config::load_config() {
        Ok(cfg) => {
            println!("Configuration:");
            println!(
                "  Kalshi API: {}",
                if cfg.kalshi_api_key_id.is_some() {
                    "Configured".green()
                } else {
                    "Not configured (using public data)".yellow()
                }
            );
            println!(
                "  Polymarket API: {}",
                "Public access (no key needed)".green()
            );
        }
        Err(_) => {
            println!("No configuration found. Run 'whale-watcher setup' to configure.");
        }
    }

    Ok(())
}

async fn watch_whales(threshold: u64, interval: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "WHALE WATCHER ACTIVE".bright_cyan().bold());
    println!(
        "Monitoring transactions exceeding: {}",
        format!("${}", format_number(threshold)).bright_green()
    );
    println!("Polling interval: {} seconds", interval);
    println!("Anomaly detection: ENABLED");
    println!();

    // Load config (optional credentials)
    let config = config::load_config().ok();

    let mut last_polymarket_trade_id: Option<String> = None;
    let mut last_kalshi_trade_id: Option<String> = None;

    let mut tick_interval = time::interval(Duration::from_secs(interval));

    loop {
        tick_interval.tick().await;

        // Check Polymarket
        match polymarket::fetch_recent_trades().await {
            Ok(mut trades) => {
                // Update last seen trade ID first
                if let Some(first_trade) = trades.first() {
                    let new_last_id = first_trade.id.clone();
                    
                    for trade in &mut trades {
                        // Skip if we've already seen this trade
                        if let Some(ref last_id) = last_polymarket_trade_id {
                            if trade.id == *last_id {
                                break;
                            }
                        }

                        let trade_value = trade.size * trade.price;
                        if trade_value >= threshold as f64 {
                            // Fetch market details
                            if let Some((title, outcome)) = polymarket::fetch_market_info(&trade.market).await {
                                trade.market_title = Some(title);
                                trade.outcome = Some(outcome);
                            }
                            print_whale_alert("Polymarket", trade, trade_value);
                        }
                    }
                    
                    last_polymarket_trade_id = Some(new_last_id);
                }
            }
            Err(e) => {
                eprintln!("{} {}", "[ERROR] Polymarket:".red(), e);
            }
        }

        // Check Kalshi
        match kalshi::fetch_recent_trades(config.as_ref()).await {
            Ok(mut trades) => {
                // Update last seen trade ID first
                if let Some(first_trade) = trades.first() {
                    let new_last_id = first_trade.trade_id.clone();
                    
                    for trade in &mut trades {
                        // Skip if we've already seen this trade
                        if let Some(ref last_id) = last_kalshi_trade_id {
                            if trade.trade_id == *last_id {
                                break;
                            }
                        }

                        // Kalshi prices are in cents, count is number of contracts
                        let trade_value = (trade.yes_price as f64 / 100.0) * trade.count as f64;
                        if trade_value >= threshold as f64 {
                            // Fetch market details
                            if let Some(title) = kalshi::fetch_market_info(&trade.ticker).await {
                                trade.market_title = Some(title);
                            }
                            print_kalshi_alert(trade, trade_value);
                        }
                    }
                    
                    last_kalshi_trade_id = Some(new_last_id);
                }
            }
            Err(e) => {
                eprintln!("{} {}", "[ERROR] Kalshi:".red(), e);
            }
        }
    }
}

fn print_whale_alert(platform: &str, trade: &polymarket::Trade, value: f64) {
    println!();
    println!(
        "{}",
        format!("[ALERT] LARGE TRANSACTION DETECTED - {}", platform)
            .bright_green()
            .bold()
    );
    println!("{}", "=".repeat(70).dimmed());
    
    // Display market title if available
    if let Some(ref title) = trade.market_title {
        println!("Market:   {}", title.bright_white().bold());
        if let Some(ref outcome) = trade.outcome {
            println!("Outcome:  {}", outcome.bright_cyan());
        }
    } else {
        println!("Market ID: {}", trade.market);
    }
    
    println!(
        "Value:    {}",
        format!("${:.2}", value).bright_yellow().bold()
    );
    println!("Price:    ${:.4} ({:.1}%)", trade.price, trade.price * 100.0);
    println!("Size:     {:.2} contracts", trade.size);
    println!("Side:     {}", trade.side.to_uppercase().bright_magenta());
    println!("Time:     {}", trade.timestamp);
    
    // Anomaly detection
    detect_anomalies(trade.price, trade.size, value);
    
    println!("Asset ID: {}", trade.asset_id.dimmed());
    println!("{}", "=".repeat(70).dimmed());
    println!();
}

fn print_kalshi_alert(trade: &kalshi::Trade, value: f64) {
    println!();
    println!(
        "{}",
        "[ALERT] LARGE TRANSACTION DETECTED - Kalshi".bright_green().bold()
    );
    println!("{}", "=".repeat(70).dimmed());
    
    // Display market title if available
    if let Some(ref title) = trade.market_title {
        println!("Market:     {}", title.bright_white().bold());
    }
    println!("Ticker:     {}", trade.ticker.bright_cyan());
    
    println!(
        "Value:      {}",
        format!("${:.2}", value).bright_yellow().bold()
    );
    println!("Yes Price:  ${:.2} ({:.1}%)", trade.yes_price, trade.yes_price);
    println!("No Price:   ${:.2} ({:.1}%)", trade.no_price, trade.no_price);
    println!("Count:      {} contracts", trade.count);
    println!("Taker Side: {}", trade.taker_side.to_uppercase().bright_magenta());
    println!("Time:       {}", trade.created_time);
    
    // Anomaly detection
    let avg_price = (trade.yes_price + trade.no_price) / 2.0;
    detect_anomalies(avg_price / 100.0, trade.count as f64, value);
    
    println!("{}", "=".repeat(70).dimmed());
    println!();
}

fn detect_anomalies(price: f64, size: f64, value: f64) {
    let mut anomalies = Vec::new();
    
    // Extreme confidence (very high or very low probability)
    if price > 0.95 {
        anomalies.push(format!("Extreme confidence bet ({:.1}% probability)", price * 100.0));
    } else if price < 0.05 {
        anomalies.push(format!("Contrarian position ({:.1}% probability)", price * 100.0));
    }
    
    // Unusual size relative to typical market activity
    if size > 100000.0 {
        anomalies.push("Exceptionally large position size".to_string());
    }
    
    // Very large single transaction
    if value > 100000.0 {
        anomalies.push(format!("Major capital deployment: ${:.0}", value));
    }
    
    // Edge case: betting on near-certain outcomes with large size
    if price > 0.90 && size > 50000.0 {
        anomalies.push("High conviction in likely outcome".to_string());
    }
    
    // Edge case: large bet on unlikely outcome (potential insider info or hedge)
    if price < 0.20 && value > 50000.0 {
        anomalies.push("Significant bet on unlikely outcome - possible hedge or information asymmetry".to_string());
    }
    
    // Display anomalies
    if !anomalies.is_empty() {
        println!();
        println!("{}", "[ANOMALY INDICATORS]".bright_red().bold());
        for anomaly in anomalies {
            println!("  - {}", anomaly.yellow());
        }
    }
}

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, ch);
    }
    result
}
