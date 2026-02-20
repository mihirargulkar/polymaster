// DOM Elements
const totalVolEl = document.getElementById('total-vol');
const activeWhalesEl = document.getElementById('active-whales'); // Now used for Total Signals
const volume24hEl = document.getElementById('volume-24h');
const potentialPnlEl = document.getElementById('potential-pnl'); // Now used for Executed Orders
const feedListEl = document.getElementById('feed-list');

// Chart (Placeholder for now, can be wired to pnl history later)
// Chart with Gradient
const ctx = document.getElementById('mainChart').getContext('2d');
const gradient = ctx.createLinearGradient(0, 0, 0, 400);
gradient.addColorStop(0, 'rgba(52, 211, 153, 0.4)'); // Emerald start
gradient.addColorStop(1, 'rgba(52, 211, 153, 0.0)'); // Transparent end

const mainChart = new Chart(ctx, {
    type: 'line',
    data: {
        labels: Array(24).fill(''),
        datasets: [{
            label: 'Cumulative PnL',
            data: Array(24).fill(0),
            borderColor: '#34d399', // Emerald 400
            backgroundColor: gradient,
            borderWidth: 2,
            tension: 0.4,
            fill: true,
            pointRadius: 0,
            pointHoverRadius: 4
        }]
    },
    options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
            legend: { display: false },
            tooltip: {
                mode: 'index',
                intersect: false,
                backgroundColor: 'rgba(15, 23, 42, 0.9)',
                titleColor: '#94a3b8',
                bodyColor: '#f8fafc',
                borderColor: 'rgba(255,255,255,0.1)',
                borderWidth: 1,
                displayColors: false,
                padding: 10,
                callbacks: {
                    label: function (context) {
                        return formatCurrency(context.parsed.y);
                    }
                }
            }
        },
        scales: { x: { display: false }, y: { display: false } },
        interaction: {
            mode: 'nearest',
            axis: 'x',
            intersect: false
        }
    }
});

async function fetchData() {
    try {
        // 1. Get Stats
        const statsRes = await fetch('/api/stats');
        const stats = await statsRes.json();

        // Main Display: Current Equity (PnL Base)
        totalVolEl.innerText = formatCurrency(stats.current_equity || 0);

        // Update PnL Delta (Today's Change)
        const deltaValEl = document.getElementById('delta-val');
        const deltaPctEl = document.getElementById('delta-pct');
        const pnlToday = stats.pnl_today || 0;
        const pnlPct = stats.pnl_pct || 0;
        const isProfit = pnlToday >= 0;

        deltaValEl.innerText = `${isProfit ? '+' : ''}${formatCurrency(pnlToday)}`;
        deltaPctEl.innerText = `(${isProfit ? '+' : ''}${pnlPct.toFixed(2)}%)`;
        deltaValEl.style.color = isProfit ? '#34d399' : '#fb7185'; // Emerald / Rose
        deltaPctEl.style.color = isProfit ? '#34d399' : '#fb7185';

        // Stats Grid
        volume24hEl.innerText = formatCurrency(stats.today_volume || 0); // Whale Volume
        activeWhalesEl.innerText = stats.today_signals || 0;
        potentialPnlEl.innerText = `${isProfit ? '+' : ''}${formatCurrency(pnlToday)}`; // Today's Profit

        // 2. Get Recent Signals (The Feed)
        const filterEnabled = document.getElementById('executed-filter').checked;
        const minValue = document.getElementById('min-value-filter')?.value || '';
        let signalsUrl = `/api/signals?limit=50${filterEnabled ? '&filter=executed' : ''}`;
        if (minValue) signalsUrl += `&minValue=${minValue}`;
        const signalsRes = await fetch(signalsUrl, { cache: 'no-store' });
        const signals = await signalsRes.json();
        renderFeed(signals);

        // 3. Update Chart with Equity History
        const chartRes = await fetch('/api/chart?limit=100');
        const chartData = await chartRes.json();
        if (chartData && Array.isArray(chartData) && chartData.length > 0) {
            mainChart.data.labels = chartData.map(d => d.hour.split('T')[1]?.substring(0, 5) || d.hour);
            mainChart.data.datasets[0].data = chartData.map(d => parseFloat(d.equity || 0));
            mainChart.update('none'); // Update without full re-render for performance
        }

    } catch (err) {
        console.error("Fetch Error:", err);
    }
}

function renderFeed(items) {
    // Only re-render if data changed? For now, we clear/re-render but add animation
    feedListEl.innerHTML = '';
    if (!items || !Array.isArray(items)) return;

    items.forEach((item, index) => {
        const el = document.createElement('div');
        el.className = 'feed-item animate-entry';
        el.style.animationDelay = `${index * 50}ms`; // Staggered animation

        const isYes = item.outcome === 'yes' || item.outcome === 'Yes';
        // const sideColor = isYes ? 'green' : 'red'; // Unused in new design

        // Handle Rust vs Python field names
        const title = item.market_title || item.asset_id || 'Unknown Market';
        const value = item.value || 0;
        const price = item.price || 0;
        const outcome = item.outcome || 'Unknown';

        const isExecuted = item.status === 'EXECUTED' || item.live_trade_id;
        el.innerHTML = `
            <div class="feed-left">
                <div class="ticker" title="${title}">
                    ${isExecuted ? '<span style="color:#34d399;font-weight:600;margin-right:4px;">TRADED</span>' : ''}${title}
                </div>
                <div class="desc">${outcome} • ${item.platform}${isExecuted ? ' → Kalshi' : ''} • <span class="time">${new Date(item.timestamp).toLocaleTimeString()}</span></div>
            </div>
            <div class="feed-right">
                <div class="price">${formatCurrency(value)}</div>
                
                ${item.settled_outcome ? `
                    <div class="meta" style="color: ${item.pnl_value > 0 ? '#34d399' : '#fb7185'}">
                        ${item.pnl_value > 0 ? 'WON' : 'LOST'} (${item.settled_outcome})
                    </div>
                ` : isExecuted ? `
                    <div class="meta" style="color: #34d399;">
                        Executed $${(item.shadow_bet_amount || 0).toFixed(2)} @ ${(price * 100).toFixed(1)}¢
                    </div>
                ` : `
                    <div class="meta text-slate-400">
                        ${(price * 100).toFixed(1)}¢ (${outcome})
                    </div>
                `}
            </div>
        `;
        el.onclick = () => openModal(item);
        feedListEl.appendChild(el);
    });
}

// Modal Functions
function openModal(item) {
    const modal = document.getElementById('trade-modal');

    // Safety check for titles/values
    const title = item.market_title || item.asset_id || 'Unknown Market';
    const value = item.value || 0;
    const price = item.price || 0;
    const outcome = item.outcome || 'Unknown';

    // Populate Fields
    document.getElementById('modal-title').innerText = title;
    document.getElementById('modal-platform').innerText = item.platform || 'Polymarket';
    document.getElementById('modal-bet').innerText = formatCurrency(value);
    document.getElementById('modal-outcome').innerText = outcome;
    document.getElementById('modal-value').innerText = formatCurrency(value); // Assuming value is bet amount
    document.getElementById('modal-price').innerText = `${(price * 100).toFixed(1)}¢`;
    document.getElementById('modal-time').innerText = new Date(item.timestamp).toLocaleString();

    // Parse Close Date from Market Context
    let closeDateStr = "Unknown";
    if (item.market_context) {
        try {
            const ctx = JSON.parse(item.market_context);
            if (ctx.expiration_date) {
                closeDateStr = new Date(ctx.expiration_date).toLocaleString();
            }
        } catch (e) { console.error("Error parsing market context", e); }
    }
    document.getElementById('modal-close-date').innerText = closeDateStr;

    // Shadow Calc
    const shadowBet = item.shadow_bet_amount || 0;
    const isSkipped = shadowBet === 0;

    // Use fixed $5 for "theoretical" shares, or 0 if skipped?
    // User wants to know why it was a loss. If it was skipped, it should say 0 shares.
    const shares = (shadowBet > 0 && price > 0) ? (shadowBet / price).toFixed(4) : "0.0000";
    document.getElementById('modal-shares').innerText = isSkipped ? "0 shares (Skipped: Reserve Limit)" : `${shares} shares`;
    document.getElementById('modal-shares').style.color = isSkipped ? '#ff3b3b' : '#fff';

    if (item.settled_outcome) {
        if (isSkipped) {
            document.getElementById('modal-result').innerText = `${item.settled_outcome} (SKIPPED)`;
            document.getElementById('modal-result').style.color = '#b3b3b3';
            document.getElementById('modal-pnl').innerText = "Insufficient Funds";
            document.getElementById('modal-pnl').style.color = '#ffb300'; // Yellow/Gold for skip
        } else {
            const isWin = item.pnl_value > 0;
            document.getElementById('modal-result').innerText = `${item.settled_outcome} (${isWin ? 'WIN' : 'LOSS'})`;
            document.getElementById('modal-result').style.color = isWin ? '#00c805' : '#ff3b3b';

            const pnl = item.pnl_value || 0;
            const pnlStr = (pnl >= 0 ? '+' : '') + formatCurrency(pnl);
            const pnlEl = document.getElementById('modal-pnl');
            pnlEl.innerText = pnlStr;
            pnlEl.style.color = pnl >= 0 ? '#00c805' : '#ff3b3b';
        }
    } else {
        document.getElementById('modal-result').innerText = isSkipped ? "SKIPPED (Insufficient Funds)" : "Pending / Active";
        document.getElementById('modal-result').style.color = isSkipped ? '#ffb300' : '#fff';
        document.getElementById('modal-pnl').innerText = isSkipped ? "No Bet Placed" : "---";
        document.getElementById('modal-pnl').style.color = '#fff';
    }

    modal.style.display = 'flex';
}

function closeModal() {
    document.getElementById('trade-modal').style.display = 'none';
}

function toggleFilter() {
    fetchData(); // Refresh everything
}

function formatCurrency(num) {
    return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' }).format(num);
}

// Init
fetchData();
setInterval(fetchData, 3000);
