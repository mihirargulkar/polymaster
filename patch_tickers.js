const sqlite3 = require('sqlite3').verbose();
const path = require('path');
const { execSync } = require('child_process');

const DB_PATH = path.join(__dirname, 'whale_alerts.db');
const db = new sqlite3.Database(DB_PATH);

async function patchTickers() {
    console.log('🩹 Patching missing tickers for Kalshi trades...');

    db.all("SELECT id, market_title FROM whale_alerts WHERE platform = 'Kalshi' AND ticker IS NULL", async (err, rows) => {
        if (err) return console.error(err);

        console.log(`🔍 Found ${rows.length} trades to patch.`);

        for (const row of rows) {
            try {
                const searchResult = execSync(`node integration/dist/cli.js search-kalshi "${row.market_title}"`, { encoding: 'utf8' });
                const results = JSON.parse(searchResult);
                if (results && results.length > 0) {
                    const ticker = results[0].ticker;
                    db.run("UPDATE whale_alerts SET ticker = ? WHERE id = ?", [ticker, row.id]);
                }
            } catch (e) {
                // Skip
            }
        }
        console.log('✅ Patching complete.');
    });
}

patchTickers();
