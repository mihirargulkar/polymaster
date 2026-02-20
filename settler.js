const sqlite3 = require('sqlite3').verbose();
const path = require('path');
const axios = require('axios');

const DB_PATH = path.join(__dirname, 'whale_alerts.db');
const db = new sqlite3.Database(DB_PATH);

// Helper to normalize strings for fuzzy matching
function normalize(str) {
    return str.toLowerCase().replace(/[^a-z0-9]/g, '');
}

async function settleTrades() {
    console.log('🧐 Fetching latest markets from Polymarket (with pagination)...');

    let polymarketData = [];
    const PAGE_SIZE = 500;
    const MAX_PAGES = 50;

    // 1. Fetch CLOSED markets (for settlement)
    for (let i = 0; i < MAX_PAGES; i++) {
        try {
            const url = `https://gamma-api.polymarket.com/markets?closed=true&limit=${PAGE_SIZE}&offset=${i * PAGE_SIZE}&order=updatedAt&ascending=false`;
            console.log(`📄 Fetching CLOSED page ${i + 1}...`);
            const response = await axios.get(url, { timeout: 20000 });
            if (response.data.length === 0) break;
            polymarketData = polymarketData.concat(response.data);
        } catch (e) {
            console.error(`❌ Failed to fetch CLOSED page ${i + 1}:`, e.message);
            break;
        }
    }

    // 2. Fetch ACTIVE markets (for ID backfilling)
    for (let i = 0; i < MAX_PAGES; i++) {
        try {
            const url = `https://gamma-api.polymarket.com/markets?closed=false&limit=${PAGE_SIZE}&offset=${i * PAGE_SIZE}&order=updatedAt&ascending=false`;
            console.log(`📄 Fetching ACTIVE page ${i + 1}...`);
            const response = await axios.get(url, { timeout: 20000 });
            if (response.data.length === 0) break;
            polymarketData = polymarketData.concat(response.data);
        } catch (e) {
            console.error(`❌ Failed to fetch ACTIVE page ${i + 1}:`, e.message);
            break;
        }
    }

    // 3. Targeted Searches for Outliers (Iran, Fed, etc.)
    const keywords = ["Iran", "Fed", "March 2026"];
    for (const kw of keywords) {
        try {
            const url = `https://gamma-api.polymarket.com/markets?q=${encodeURIComponent(kw)}&limit=100`;
            console.log(`🔎 Targeted search for '${kw}'...`);
            const response = await axios.get(url, { timeout: 20000 });
            polymarketData = polymarketData.concat(response.data);
        } catch (e) {
            console.error(`❌ Failed targeted search for '${kw}':`, e.message);
        }
    }

    console.log(`📊 Retrieved ${polymarketData.length} total markets for potential matching.`);

    if (polymarketData.length > 0) {
        console.log('📝 Sample Market Keys:', Object.keys(polymarketData[0]));
    }

    // Build a map of normalized titles to result objects
    const marketMap = new Map();
    polymarketData.forEach(m => {
        try {
            const normTitle = normalize(m.question || "");

            // Determine if settled
            const isClosed = (m.closed === true || m.closed === 'true');
            const isResolved = (m.umaResolutionStatus === 'resolved');
            const isSettled = isClosed && isResolved;

            let winner = null;
            if (isSettled && m.outcomes && m.outcomePrices) {
                const outcomes = typeof m.outcomes === 'string' ? JSON.parse(m.outcomes) : m.outcomes;
                const prices = typeof m.outcomePrices === 'string' ? JSON.parse(m.outcomePrices) : m.outcomePrices;
                const winIndex = prices.findIndex(p => p === "1" || p === 1 || p === "1.0");
                if (winIndex !== -1) winner = outcomes[winIndex].toUpperCase();
            }

            // Store ID and potential winner
            marketMap.set(normTitle, {
                id: m.id,
                settled: isSettled,
                result: winner
            });

        } catch (e) {
            // Skip malformed entries
        }
    });

    console.log(`✅ Indexed ${marketMap.size} markets (Active & Closed).`);

    // Process ALL trades to ensure IDs are backfilled
    db.all("SELECT * FROM whale_alerts", async (err, rows) => {
        if (err) {
            console.error('Error fetching open trades:', err.message);
            process.exit(1);
        }

        console.log(`📊 Checking ${rows.length} open trades against index...`);
        let settledCount = 0;

        // Process sequentially to avoid DB locking issues
        for (const row of rows) {
            const normRowTitle = normalize(row.market_title);
            const match = marketMap.get(normRowTitle);

            if (match) {
                const isWin = (row.outcome.toUpperCase() === match.result);
                const status = isWin ? 'WIN' : 'LOSS';

                await new Promise(resolve => {
                    db.run(
                        "UPDATE whale_alerts SET status = ?, market_id = ? WHERE id = ?",
                        [status, match.id, row.id],
                        (err) => {
                            if (!err) {
                                console.log(`✅ Settled [${row.id}] ${row.market_title.substring(0, 30)}... as ${status} (Result: ${match.result}, ID: ${match.id})`);
                                settledCount++;
                            } else {
                                console.error(`❌ Failed to update [${row.id}]:`, err.message);
                            }
                            resolve();
                        }
                    );
                });
            }
        }

        console.log(`\n🎉 Settlement run complete! Total settled: ${settledCount}/${rows.length}`);
        db.close();
    });
}

settleTrades();
