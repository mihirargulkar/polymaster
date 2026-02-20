CREATE TABLE whale_alerts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                platform TEXT NOT NULL,
                market_title TEXT NOT NULL,
                whale_name TEXT NOT NULL,
                action TEXT NOT NULL,
                outcome TEXT NOT NULL,
                price REAL NOT NULL,
                size REAL NOT NULL,
                value REAL NOT NULL,
                win_rate REAL,
                status TEXT DEFAULT 'OPEN',
                pnl_theoretical REAL
            , ticker TEXT, market_id TEXT);
CREATE TABLE sqlite_sequence(name,seq);
CREATE TABLE orders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                signal_id INTEGER,
                kalshi_ticker TEXT,
                side TEXT,
                price_cents INTEGER,
                count INTEGER,
                status TEXT,
                kalshi_order_id TEXT,
                timestamp TEXT,
                FOREIGN KEY(signal_id) REFERENCES whale_alerts(id)
            );
CREATE TABLE portfolio_snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT,
                balance_cents INTEGER,
                portfolio_value_cents INTEGER,
                total_equity_cents INTEGER
            );
CREATE TABLE alerts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            platform TEXT NOT NULL,
            alert_type TEXT NOT NULL,
            action TEXT NOT NULL,
            category TEXT,
            subcategory TEXT,
            value REAL NOT NULL,
            price REAL NOT NULL,
            size REAL NOT NULL,
            market_title TEXT,
            market_id TEXT,
            outcome TEXT,
            wallet_hash TEXT,
            wallet_id TEXT,
            timestamp TEXT NOT NULL,
            market_context TEXT,
            wallet_activity TEXT,
            created_at INTEGER DEFAULT (strftime('%s', 'now'))
        , status TEXT DEFAULT 'OPEN', settled_outcome TEXT, pnl_value REAL, shadow_bet_amount REAL, shadow_active INTEGER DEFAULT 0);
CREATE INDEX idx_alerts_wallet_hash ON alerts(wallet_hash);
CREATE INDEX idx_alerts_timestamp ON alerts(created_at);
CREATE INDEX idx_alerts_category ON alerts(category);
CREATE INDEX idx_alerts_platform ON alerts(platform);
CREATE TABLE wallet_memory (
            wallet_hash TEXT NOT NULL,
            wallet_id TEXT NOT NULL,
            market_title TEXT,
            market_id TEXT,
            outcome TEXT,
            action TEXT,
            value REAL NOT NULL,
            price REAL NOT NULL,
            platform TEXT NOT NULL,
            category TEXT,
            seen_at INTEGER NOT NULL,
            PRIMARY KEY (wallet_hash, market_id, seen_at)
        );
CREATE INDEX idx_wallet_memory_hash ON wallet_memory(wallet_hash);
CREATE INDEX idx_wallet_memory_seen ON wallet_memory(seen_at);
CREATE TABLE metadata (
            key TEXT PRIMARY KEY,
            value TEXT
        );
