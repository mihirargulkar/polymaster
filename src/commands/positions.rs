use colored::*;

use crate::execution::kalshi::KalshiExecutor;

fn resolve_pem(input: &str) -> String {
    if input.starts_with('/') || input.starts_with('.') || input.contains('/') {
        std::fs::read_to_string(input).unwrap_or_else(|_| input.to_string())
    } else {
        input.to_string()
    }
}

pub async fn show_positions() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "KALSHI POSITIONS".bright_cyan().bold());
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

    let (key_id, private_key_input) = match (&config.kalshi_api_key_id, &config.kalshi_private_key) {
        (Some(k), Some(p)) => (k.clone(), p.clone()),
        _ => {
            println!(
                "{}",
                "Kalshi API not configured. Run 'wwatcher setup' to add credentials.".red()
            );
            return Ok(());
        }
    };

    let private_key_pem = resolve_pem(&private_key_input);
    let executor = match KalshiExecutor::new(key_id, &private_key_pem, config.kalshi_is_demo) {
        Ok(ex) => ex,
        Err(e) => {
            println!("{} Failed to create Kalshi client: {}", "[ERROR]".red(), e);
            return Ok(());
        }
    };

    let balance = executor.get_balance().await.unwrap_or(0);
    println!("Balance: ${:.2}", balance as f64 / 100.0);
    println!();

    match executor.get_positions().await {
        Ok(positions) => {
            if positions.is_empty() {
                println!("{}", "No open positions.".dimmed());
            } else {
                println!("Open positions ({}):", positions.len());
                for (ticker, side, count) in positions {
                    println!("  • {} {} {} contracts", ticker.bright_white(), side.green(), count);
                }
            }
        }
        Err(e) => {
            println!("{} Failed to fetch positions: {}", "[ERROR]".red(), e);
        }
    }

    Ok(())
}
