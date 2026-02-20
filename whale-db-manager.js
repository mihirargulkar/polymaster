const sqlite3 = require('sqlite3').verbose();
const path = require('path');

const DB_PATH = path.join(__dirname, 'whale_alerts.db');

class WhaleDatabase {
    constructor() {
        this.db = new sqlite3.Database(DB_PATH, (err) => {
            if (err) {
                console.error('❌ Database connection error:', err.message);
            } else {
                console.log('📂 Connected to Whale Database.');
                this.initDB();
            }
        });
    }

    initDB() {
        const schema = `
            CREATE TABLE IF NOT EXISTS whale_alerts (
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
            );
        `;
        this.db.run(schema, (err) => {
            if (err) {
                console.error('❌ Schema initialization error:', err.message);
            }
        });
    }

    async logAlert(data) {
        return new Promise((resolve, reject) => {
            const sql = `
                INSERT INTO whale_alerts (
                    timestamp, platform, market_title, whale_name, action, 
                    outcome, price, size, value, win_rate, pnl_theoretical, ticker
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            `;

            const params = [
                data.timestamp || new Date().toISOString(),
                data.platform,
                data.market_title,
                data.whale_profile ? (data.whale_profile.username || data.wallet_id) : data.wallet_id,
                data.action,
                data.outcome,
                data.price,
                data.size,
                data.value,
                (data.whale_profile && data.whale_profile.win_rate !== null && data.whale_profile.win_rate !== undefined) ? (data.whale_profile.win_rate * 100).toFixed(1) + '%' : 'N/A',
                data.action === 'BUY' ? (data.size * (1 - data.price)) : 0,
                data.ticker || null
            ];

            this.db.run(sql, params, function (err) {
                if (err) {
                    console.error('❌ Error logging alert to DB:', err.message);
                    reject(err);
                } else {
                    console.log(`✅ Alert logged to DB (ID: ${this.lastID})`);
                    resolve(this.lastID);
                }
            });
        });
    }

    close() {
        this.db.close();
    }
}

module.exports = new WhaleDatabase();
