const express = require('express');
const sqlite3 = require('sqlite3').verbose();
const path = require('path');
const cors = require('cors');

const app = express();
const PORT = 3000;

// Connect to REAL Python Bot Database
// Connect to REAL Python Bot Database
const DB_PATH = path.join(__dirname, '../whale_alerts.db');
const db = new sqlite3.Database(DB_PATH, sqlite3.OPEN_READWRITE, (err) => {
    if (err) {
        console.error('❌ Error connecting to database:', err.message);
        console.error('   Make sure python_bot has initialized the DB first!');
    } else {
        console.log('📂 Connected to Copy Trader DB at:', DB_PATH);
        // Ensure WAL mode is active for this connection too
        db.run('PRAGMA journal_mode=WAL;');
    }
});

app.use(express.json());

app.use(cors());

// Disable caching for real-time data
app.use((req, res, next) => {
    res.set('Cache-Control', 'no-store');
    next();
});

app.use(express.static(path.join(__dirname, 'public')));

// API: Get Recent Whale Signals (Polymarket)
app.get('/api/signals', (req, res) => {
    const limit = parseInt(req.query.limit, 10) || 50;
    const filter = req.query.filter; // e.g., 'executed'
    const minValue = parseFloat(req.query.minValue);

    let query = "SELECT *, market_id as asset_id FROM alerts";
    const conditions = [];
    const params = [];

    if (filter === 'executed') {
        conditions.push("shadow_bet_amount > 0");
    }
    if (!isNaN(minValue) && minValue > 0) {
        conditions.push("value >= ?");
        params.push(minValue);
    }
    if (conditions.length > 0) {
        query += " WHERE " + conditions.join(" AND ");
    }
    query += " ORDER BY id DESC LIMIT ?";
    params.push(limit);

    db.all(query, params, (err, rows) => {
        if (err) {
            res.status(500).json({ error: err.message });
            return;
        }
        res.json(rows);
    });
});

// API: Get Executed Orders (Kalshi)
app.get('/api/orders', (req, res) => {
    const limit = req.query.limit || 50;
    db.all(`SELECT * FROM orders ORDER BY id DESC LIMIT ?`, [limit], (err, rows) => {
        if (err) {
            res.status(500).json({ error: err.message });
            return;
        }
        res.json(rows);
    });
});

// API: Get Executed Trades Table (CSV Style)
app.get('/api/executed-trades', (req, res) => {
    // Return all executed trades, ordered chronologically (Ascending)
    // Filter: shadow_bet_amount > 0 means we took the trade
    const query = `
        SELECT 
            id, 
            timestamp, 
            market_title, 
            outcome, 
            price, 
            shadow_bet_amount, 
            status, 
            settled_outcome, 
            pnl_value 
        FROM alerts 
        WHERE (status = 'EXECUTED' OR live_trade_id IS NOT NULL)
        ORDER BY id ASC
    `;

    db.all(query, [], (err, rows) => {
        if (err) {
            res.status(500).json({ error: err.message });
            return;
        }
        res.json(rows);
    });
});

// API: Get Historical Equity Data for Chart
// API: Get Cumulative PnL by Trade
app.get('/api/chart', (req, res) => {
    // Get all settled trades that were not skipped
    const query = `
        SELECT timestamp, pnl_value 
        FROM alerts 
        WHERE settled_outcome IS NOT NULL 
          And status = 'SETTLED'
          AND shadow_bet_amount > 0 
        ORDER BY id ASC
    `;

    db.all(query, [], (err, rows) => {
        if (err) {
            res.json([]);
            return;
        }

        // Start from initial bankroll or 0? 
        // User asked for "PnL by trade". A cumulative PnL curve is best.
        let runningTotal = 0;
        const chartData = rows.map(r => {
            runningTotal += r.pnl_value || 0;
            return {
                hour: r.timestamp,
                equity: runningTotal.toFixed(2)
            };
        });

        // If no trades, return empty or baseline
        if (chartData.length === 0) {
            chartData.push({ hour: new Date().toISOString(), equity: "0.00" });
        }

        res.json(chartData);
    });
});

// API: Get Dashboard Stats (Aggregated)
app.get('/api/stats', (req, res) => {
    const startOfDay = new Date();
    startOfDay.setHours(0, 0, 0, 0);
    const startOfDayISO = startOfDay.toISOString();

    const stats = {};

    // 1. Get Today's Whale Volume
    db.get("SELECT sum(value) as volume FROM alerts WHERE timestamp >= ?", [startOfDayISO], (err, row) => {
        stats.today_volume = row ? (row.volume || 0) : 0;

        // 2. Get Signals Count
        db.get("SELECT count(*) as count FROM alerts WHERE timestamp >= ?", [startOfDayISO], (err, row) => {
            stats.today_signals = row ? row.count : 0;

            // 3. Get Current Equity (PnL Base)
            db.get("SELECT total_equity_cents FROM portfolio_snapshots ORDER BY id DESC LIMIT 1", [], (err, row) => {
                const currentEquity = row ? (row.total_equity_cents / 100) : 0;
                stats.current_equity = currentEquity;

                // 4. Get Equity at start of day for PnL Delta
                db.get("SELECT total_equity_cents FROM portfolio_snapshots WHERE timestamp >= ? ORDER BY id ASC LIMIT 1", [startOfDayISO], (err, row) => {
                    const startEquity = row ? (row.total_equity_cents / 100) : currentEquity;
                    stats.pnl_today = currentEquity - startEquity;
                    stats.pnl_pct = startEquity > 0 ? ((stats.pnl_today / startEquity) * 100) : 0;

                    // 5. Executed Orders (from alerts table, where Rust watcher marks them)
                    db.get("SELECT count(*) as count FROM alerts WHERE status = 'EXECUTED' OR live_trade_id IS NOT NULL", [], (err, row) => {
                        stats.executed_orders = row ? row.count : 0;
                        res.json(stats);
                    });
                });
            });
        });
    });
});

app.listen(PORT, () => {
    console.log(`🚀 Dashboard running at http://localhost:${PORT}`);
    console.log(`   Serving real-time data from ${DB_PATH}`);
});
