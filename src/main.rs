mod alerts;
mod categories;
mod commands;
mod config;
mod db;
mod platforms;
mod types;
mod whale_profile;
mod ws;
mod execution;

use std::sync::{Arc, Mutex};

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "wwatcher")]
#[command(about = "Whale Watcher - Monitor large transactions on Polymarket and Kalshi", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Watch for large transactions (uses config threshold from setup, or $25,000 default)
    Watch {
        /// Minimum transaction size to alert on (in USD). Overrides config.
        #[arg(short, long)]
        threshold: Option<u64>,

        /// Polling interval in seconds
        #[arg(short, long, default_value = "5")]
        interval: u64,
    },
    /// View alert history
    History {
        /// Number of alerts to show (default: 20)
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Filter by platform: polymarket, kalshi, or all (default: all)
        #[arg(short, long, default_value = "all")]
        platform: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Configure API credentials
    Setup,
    /// Show current configuration
    Status,
    /// Test alert sound
    TestSound,
    /// Test webhook notification
    TestWebhook,
    /// Test whale profile fetching
    TestProfile {
        /// Wallet ID to fetch profile for
        wallet_id: String,
    },
    /// Show Kalshi open positions and balance
    Positions,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize database for commands that need it
    let conn = db::open_db()?;

    // Migrate old JSONL history on first run (Disabled to keep JSONL live for autopilot)
    // db::migrate_jsonl_if_exists(&conn);

    match cli.command {
        Commands::Setup => {
            commands::setup::setup_config().await?;
        }
        Commands::Status => {
            commands::status::show_status(&conn).await?;
        }
        Commands::Watch {
            threshold,
            interval,
        } => {
            let effective_threshold = threshold.unwrap_or_else(|| {
                config::load_config()
                    .ok()
                    .map(|c| c.threshold)
                    .unwrap_or(25000)
            });
            let conn = Arc::new(Mutex::new(conn));
            commands::watch::watch_whales(effective_threshold, interval, conn).await?;
        }
        Commands::History {
            limit,
            platform,
            json,
        } => {
            alerts::history::show_alert_history(limit, &platform, json, &conn)?;
        }
        Commands::TestSound => {
            commands::test::test_sound().await?;
        }
        Commands::TestWebhook => {
            commands::test::test_webhook(&conn).await?;
        }
        Commands::TestProfile { wallet_id } => {
            commands::test::test_profile(&wallet_id).await?;
        }
        Commands::Positions => {
            commands::positions::show_positions().await?;
        }
    }

    Ok(())
}
