use std::fs::OpenOptions;
use std::io::Write;
use colored::*;
use rusqlite::Connection;

use super::AlertData;
use crate::db;

/// Log an alert to the SQLite database and JSONL file
pub fn log_alert(alert: &AlertData, conn: &Connection) {
    let alert_json = super::build_alert_payload(alert, false);

    let wallet_activity_json = alert_json.get("wallet_activity").map(|v| v.to_string());
    let market_context_json = alert_json.get("market_context").map(|v| v.to_string());

    // JSONL Logging
    if let Some(config_dir) = dirs::config_dir() {
        let jsonl_path = config_dir.join("wwatcher").join("alert_history.jsonl");
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&jsonl_path)
        {
            if let Ok(line) = serde_json::to_string(&alert_json) {
                let _ = writeln!(file, "{}", line);
            }
        }
    }

    // Database Logging
    db::insert_alert(
        conn,
        alert.platform,
        alert.alert_type(),
        &alert.side.to_uppercase(),
        alert.value,
        alert.price,
        alert.size,
        alert.market_title,
        None,
        alert.outcome,
        alert.wallet_id,
        alert.timestamp,
        market_context_json.as_deref(),
        wallet_activity_json.as_deref(),
    );
}

pub fn show_alert_history(
    limit: usize,
    platform_filter: &str,
    as_json: bool,
    conn: &Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    let alerts = db::query_alerts(conn, limit, platform_filter)?;

    if alerts.is_empty() {
        println!("No alerts found matching filters.");
        println!(
            "Run {} to start monitoring and logging alerts.",
            "wwatcher watch".bright_cyan()
        );
        return Ok(());
    }

    if as_json {
        println!("{}", serde_json::to_string_pretty(&alerts)?);
    } else {
        println!("{}", "ALERT HISTORY".bright_cyan().bold());
        println!("Showing {} most recent alerts", alerts.len());
        if platform_filter != "all" {
            println!("Platform filter: {}", platform_filter);
        }
        println!();

        for (i, alert) in alerts.iter().enumerate() {
            let platform = alert
                .get("platform")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            let alert_type = alert
                .get("alert_type")
                .and_then(|v| v.as_str())
                .unwrap_or("UNKNOWN");
            let action = alert
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("UNKNOWN");
            let value = alert.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let timestamp = alert
                .get("timestamp")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            let market_title = alert
                .get("market_title")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown market");
            let outcome = alert.get("outcome").and_then(|v| v.as_str());

            let header = format!("#{} | {} | {}", i + 1, platform, alert_type);
            println!("{}", header.bright_yellow());
            println!("Time:   {}", timestamp.dimmed());
            println!("Market: {}", market_title);
            if let Some(out) = outcome {
                println!("Outcome: {}", out);
            }
            println!("Action: {} | Value: ${:.2}", action, value);

            if let Some(wallet_activity) = alert.get("wallet_activity") {
                if let Some(txns_hour) = wallet_activity
                    .get("transactions_last_hour")
                    .and_then(|v| v.as_u64())
                {
                    if txns_hour > 1 {
                        println!("Wallet: {} txns in last hour", txns_hour);
                    }
                }
            }

            println!();
        }

        let total = db::alert_count(conn);
        println!(
            "Total alerts in database: {}",
            total.to_string().bright_white()
        );
        println!(
            "View as JSON: {} --json",
            "wwatcher history".bright_cyan()
        );
        println!(
            "Filter by platform: {} --platform polymarket",
            "wwatcher history".bright_cyan()
        );
    }

    Ok(())
}
