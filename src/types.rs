// Shared types and utilities across modules

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct WalletTracker {
    // Map of wallet/account ID to list of transaction values and timestamps
    transactions: HashMap<String, Vec<(f64, u64)>>,
}

impl WalletTracker {
    pub fn new() -> Self {
        Self {
            transactions: HashMap::new(),
        }
    }

    pub fn record_transaction(&mut self, wallet_id: &str, value: f64) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.transactions
            .entry(wallet_id.to_string())
            .or_default()
            .push((value, timestamp));

        // Keep only last 24 hours of data
        self.cleanup_old_transactions();
    }

    pub fn get_activity(&self, wallet_id: &str) -> WalletActivity {
        if let Some(txns) = self.transactions.get(wallet_id) {
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            // Filter to last hour
            let hour_txns: Vec<&(f64, u64)> = txns
                .iter()
                .filter(|(_, ts)| current_time - ts < 3600)
                .collect();

            // Filter to last 24 hours
            let day_txns: Vec<&(f64, u64)> = txns
                .iter()
                .filter(|(_, ts)| current_time - ts < 86400)
                .collect();

            let total_value_hour: f64 = hour_txns.iter().map(|(v, _)| v).sum();
            let total_value_day: f64 = day_txns.iter().map(|(v, _)| v).sum();

            WalletActivity {
                transactions_last_hour: hour_txns.len(),
                transactions_last_day: day_txns.len(),
                total_value_hour,
                total_value_day,
                is_repeat_actor: hour_txns.len() > 1,
                is_heavy_actor: day_txns.len() >= 5,
            }
        } else {
            WalletActivity::default()
        }
    }

    fn cleanup_old_transactions(&mut self) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        for txns in self.transactions.values_mut() {
            txns.retain(|(_, ts)| current_time - ts < 86400);
        }

        // Remove wallets with no recent activity
        self.transactions.retain(|_, txns| !txns.is_empty());
    }
}

#[derive(Debug, Clone)]
pub struct WalletActivity {
    pub transactions_last_hour: usize,
    pub transactions_last_day: usize,
    pub total_value_hour: f64,
    pub total_value_day: f64,
    pub is_repeat_actor: bool,
    pub is_heavy_actor: bool,
}

impl Default for WalletActivity {
    fn default() -> Self {
        Self {
            transactions_last_hour: 0,
            transactions_last_day: 0,
            total_value_hour: 0.0,
            total_value_day: 0.0,
            is_repeat_actor: false,
            is_heavy_actor: false,
        }
    }
}
