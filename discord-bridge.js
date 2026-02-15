const express = require('express');
const axios = require('axios');
const app = express();
app.use(express.json());

// YOUR DISCORD WEBHOOK URL
const DISCORD_WEBHOOK_URL = 'https://discord.com/api/webhooks/1471972284020428984/WJvx_yiUR4jF9S1PsPJBi1JtpoINFAG3sWHYTHQF8mXRszSH2wBxr-PrqPLpovxW_b0u';

app.post('/webhook/whale-alerts', async (req, res) => {
  try {
    const data = req.body;
    console.log(`[${new Date().toISOString()}] Received alert: ${data.market_title}`);

    // Determine color based on action (BUY: Green, SELL: Red)
    const color = data.action === 'BUY' ? 0x27ae60 : 0xe74c3c;

    // Format large numbers
    const formatUSD = (val) => val ? `$${Number(val).toLocaleString()}` : 'N/A';

    // Create the embed
    const embed = {
      title: `${data.alert_type} - ${data.platform}`,
      url: data.platform === 'Polymarket' ? 'https://polymarket.com' : 'https://kalshi.com',
      description: `**${data.market_title}**\n${data.action}ing **${data.outcome}** shares`,
      color: color,
      fields: [
        { name: 'Value', value: formatUSD(data.value), inline: true },
        { name: 'Price', value: `${(data.price * 100).toFixed(1)}%`, inline: true },
        { name: 'Size', value: `${Number(data.size).toLocaleString()} shares`, inline: true },
      ],
      timestamp: data.timestamp || new Date().toISOString(),
      footer: {
        text: 'Polymaster Whale Watcher'
      }
    };

    // Add Wallet Activity if available
    if (data.wallet_activity) {
      let walletDesc = `\`${data.wallet_id.substring(0, 10)}...\`\n`;
      walletDesc += `Txns (1h/24h): ${data.wallet_activity.transactions_last_hour} / ${data.wallet_activity.transactions_last_day}\n`;
      walletDesc += `Vol (24h): ${formatUSD(data.wallet_activity.total_value_day)}`;

      embed.fields.push({ name: 'Wallet Activity', value: walletDesc, inline: false });

      if (data.wallet_activity.is_heavy_actor) {
        embed.footer.text = '⚠️ HEAVY ACTOR (5+ txns in 24h)';
      } else if (data.wallet_activity.is_repeat_actor) {
        embed.footer.text = '👤 REPEAT ACTOR (2+ txns in 1h)';
      }
    }

    // Add Whale Profile if available (Polymarket only)
    if (data.whale_profile) {
      const profile = data.whale_profile;
      let profileDesc = `Rank: #${profile.leaderboard_rank || 'N/A'}\n`;
      profileDesc += `Win Rate: ${(profile.win_rate * 100).toFixed(1)}%\n`;
      profileDesc += `Profit: ${formatUSD(profile.leaderboard_profit)}`;

      embed.fields.push({ name: 'Whale Profile', value: profileDesc, inline: true });
    }

    // Resolve ticker for Kalshi to provide quick trade command
    let quickTradeCommand = null;
    if (data.platform === 'Kalshi') {
      try {
        // Try to find the exact ticker using our new search tool
        const { execSync } = require('child_process');
        const searchResult = execSync(`node integration/dist/cli.js search-kalshi "${data.market_title}"`, { encoding: 'utf8' });
        const results = JSON.parse(searchResult);
        if (results && results.length > 0) {
          const ticker = results[0].ticker;
          quickTradeCommand = `\`node integration/dist/cli.js buy ${ticker} 10\``;
        }
      } catch (e) {
        // Fallback or ignore
      }
    }

    // Send to Discord
    const message = {
      embeds: [embed]
    };

    if (quickTradeCommand) {
      embed.fields.push({ name: 'Quick Trade (Copy/Paste)', value: quickTradeCommand, inline: false });
    }

    await axios.post(DISCORD_WEBHOOK_URL, message);

    res.sendStatus(200);
  } catch (err) {
    console.error('Error forwarding to Discord:', err.message);
    res.status(500).send(err.message);
  }
});

const PORT = process.env.PORT || 3000;
app.listen(PORT, () => {
  console.log(`\n🚀 Discord Bridge running at http://localhost:${PORT}/webhook/whale-alerts`);
  console.log(`Configure wwatcher to use this URL in 'wwatcher setup'`);
});
