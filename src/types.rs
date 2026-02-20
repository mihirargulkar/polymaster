// Shared types and utilities across modules

use std::collections::{HashMap, HashSet};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, params};

use crate::db;

// ─── Wallet Memory (SQLite-backed with in-memory hot cache) ─────────

pub struct WalletTracker {
    // In-memory cache of known wallet hashes (refreshed periodically)
    known_hashes: HashSet<String>,
    last_cache_refresh: Instant,
    // Fallback in-memory tracker for basic activity (1h/24h windows)
    transactions: HashMap<String, Vec<(f64, u64)>>,
}

impl WalletTracker {
    pub fn new() -> Self {
        Self {
            known_hashes: HashSet::new(),
            last_cache_refresh: Instant::now(),
            transactions: HashMap::new(),
        }
    }

    /// Check if a wallet is known from recent activity (O(1) check)
    pub fn is_known(&self, wallet_id: &str) -> bool {
        let hash = db::wallet_hash(wallet_id);
        self.known_hashes.contains(&hash)
    }

    /// Record a transaction into wallet_memory table and in-memory tracker
    pub fn record_transaction(&mut self, wallet_id: &str, value: f64) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // In-memory tracker for real-time 1h/24h stats
        self.transactions
            .entry(wallet_id.to_string())
            .or_default()
            .push((value, timestamp));

        self.cleanup_old_transactions();
    }

    /// Record a transaction into the SQLite wallet_memory table
    pub fn record_to_db(
        &mut self,
        conn: &Connection,
        wallet_id: &str,
        market_title: Option<&str>,
        market_id: Option<&str>,
        outcome: Option<&str>,
        action: &str,
        value: f64,
        price: f64,
        platform: &str,
    ) {
        let hash = db::wallet_hash(wallet_id);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let result = conn.execute(
            "INSERT OR REPLACE INTO wallet_memory
             (wallet_hash, wallet_id, market_title, market_id, outcome, action, value, price, platform, seen_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![hash, wallet_id, market_title, market_id, outcome, action, value, price, platform, now],
        );

        if let Err(e) = result {
            eprintln!("Warning: Failed to record wallet memory: {}", e);
        }

        // Add to hot cache
        self.known_hashes.insert(hash);
    }

    /// Query wallet history from SQLite (last 12h)
    pub fn get_wallet_history(&self, conn: &Connection, wallet_id: &str) -> Vec<WalletMemoryEntry> {
        let hash = db::wallet_hash(wallet_id);
        let mut entries = Vec::new();

        let result = conn.prepare(
            "SELECT wallet_id, market_title, market_id, outcome, action, value, price, platform, seen_at
             FROM wallet_memory
             WHERE wallet_hash = ?1 AND seen_at > (strftime('%s', 'now') - 43200)
             ORDER BY seen_at DESC"
        );

        if let Ok(mut stmt) = result {
            let rows = stmt.query_map(params![hash], |row| {
                Ok(WalletMemoryEntry {
                    wallet_id: row.get(0)?,
                    market_title: row.get(1)?,
                    market_id: row.get(2)?,
                    outcome: row.get(3)?,
                    action: row.get(4)?,
                    value: row.get(5)?,
                    price: row.get(6)?,
                    platform: row.get(7)?,
                    seen_at: row.get(8)?,
                })
            });

            if let Ok(rows) = rows {
                for row in rows.flatten() {
                    entries.push(row);
                }
            }
        }

        entries
    }

    /// Classify the returning whale scenario
    pub fn classify_whale_return(
        &self,
        conn: &Connection,
        wallet_id: &str,
        current_market_id: Option<&str>,
        current_outcome: Option<&str>,
    ) -> Option<WhaleReturnScenario> {
        if !self.is_known(wallet_id) {
            return None;
        }

        let history = self.get_wallet_history(conn, wallet_id);
        if history.is_empty() {
            return None;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let total_volume: f64 = history.iter().map(|e| e.value).sum();
        let total_txns = history.len();

        // Check for same market interactions
        if let Some(market_id) = current_market_id {
            let same_market: Vec<&WalletMemoryEntry> = history
                .iter()
                .filter(|e| e.market_id.as_deref() == Some(market_id))
                .collect();

            if !same_market.is_empty() {
                if let Some(current_out) = current_outcome {
                    // Check if same outcome (doubling down) or opposite (flip)
                    let same_side = same_market
                        .iter()
                        .any(|e| e.outcome.as_deref() == Some(current_out));
                    let opposite_side = same_market
                        .iter()
                        .any(|e| e.outcome.as_deref() != Some(current_out) && e.outcome.is_some());

                    if opposite_side {
                        let prev = &same_market[0];
                        let hours_ago = (now - prev.seen_at) as f64 / 3600.0;
                        return Some(WhaleReturnScenario::Flip {
                            previous_outcome: prev.outcome.clone().unwrap_or_default(),
                            previous_value: prev.value,
                            hours_ago,
                            total_12h_volume: total_volume,
                            total_12h_txns: total_txns,
                        });
                    } else if same_side {
                        let prev_total: f64 = same_market.iter().map(|e| e.value).sum();
                        return Some(WhaleReturnScenario::DoublingDown {
                            previous_value: prev_total,
                            previous_txns: same_market.len(),
                            total_12h_volume: total_volume,
                            total_12h_txns: total_txns,
                        });
                    }
                }
            }
        }

        // General known whale
        Some(WhaleReturnScenario::KnownWhale {
            total_12h_volume: total_volume,
            total_12h_txns: total_txns,
            previous_entries: history,
        })
    }

    /// Refresh the in-memory hash cache from DB (every 5 minutes)
    pub fn maybe_refresh_cache(&mut self, conn: &Connection) {
        if self.last_cache_refresh.elapsed().as_secs() < 300 {
            return;
        }

        let result = conn.prepare(
            "SELECT DISTINCT wallet_hash FROM wallet_memory
             WHERE seen_at > (strftime('%s', 'now') - 43200)"
        );

        if let Ok(mut stmt) = result {
            let rows = stmt.query_map([], |row| {
                let hash: String = row.get(0)?;
                Ok(hash)
            });

            if let Ok(rows) = rows {
                self.known_hashes.clear();
                for row in rows.flatten() {
                    self.known_hashes.insert(row);
                }
            }
        }

        self.last_cache_refresh = Instant::now();
    }

    /// Get real-time activity stats (from in-memory tracker)
    pub fn get_activity(&self, wallet_id: &str) -> WalletActivity {
        if let Some(txns) = self.transactions.get(wallet_id) {
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let hour_txns: Vec<&(f64, u64)> = txns
                .iter()
                .filter(|(_, ts)| current_time - ts < 3600)
                .collect();

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

        self.transactions.retain(|_, txns| !txns.is_empty());
    }
}

// ─── Data Structures ─────────────────────────────────────────────────

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

#[derive(Debug, Clone)]
pub struct WalletMemoryEntry {
    #[allow(dead_code)]
    pub wallet_id: String,
    pub market_title: Option<String>,
    pub market_id: Option<String>,
    pub outcome: Option<String>,
    pub action: Option<String>,
    pub value: f64,
    pub price: f64,
    #[allow(dead_code)]
    pub platform: String,
    pub seen_at: i64,
}

#[derive(Debug, Clone)]
pub enum WhaleReturnScenario {
    /// Same market, same outcome — whale is adding to position
    DoublingDown {
        previous_value: f64,
        previous_txns: usize,
        total_12h_volume: f64,
        total_12h_txns: usize,
    },
    /// Same market, opposite outcome — whale changed their mind
    Flip {
        previous_outcome: String,
        previous_value: f64,
        hours_ago: f64,
        total_12h_volume: f64,
        total_12h_txns: usize,
    },
    /// Any previous activity in last 12h
    KnownWhale {
        total_12h_volume: f64,
        total_12h_txns: usize,
        previous_entries: Vec<WalletMemoryEntry>,
    },
}
