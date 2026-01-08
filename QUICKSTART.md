# Quick Start Guide

Get started with Whale Watcher in under 2 minutes.

## Step 1: Build the Tool

```bash
cd polymaster
cargo build --release
```

The binary will be at `target/release/whale-watcher`

## Step 2: Run Without Setup (Optional Configuration)

You can start watching immediately without any setup:

```bash
./target/release/whale-watcher watch
```

This will:
- Watch for transactions over $25,000
- Check every 5 seconds
- Use public API endpoints (no auth needed)
- Show market titles and bet details
- Detect anomalies in trading patterns

## Step 3: Customize Your Watch

```bash
# Watch for $50k+ trades
./target/release/whale-watcher watch -t 50000

# Check every 30 seconds instead
./target/release/whale-watcher watch -i 30

# Both together - $100k threshold, check every minute
./target/release/whale-watcher watch -t 100000 -i 60
```

## Step 4: Optional - Add Kalshi Authentication

If you want authenticated Kalshi access:

```bash
./target/release/whale-watcher setup
```

Follow the prompts to add your Kalshi API credentials.

## API Key Information

### Polymarket
- **No API key needed**
- Uses public data endpoint: `https://data-api.polymarket.com`
- Works out of the box

### Kalshi
- **Public endpoint available** (no auth needed)
- **Optional auth**: For higher rate limits
  - Create account: https://kalshi.com
  - Generate keys: https://kalshi.com/profile/api-keys
  - Add via: `./target/release/whale-watcher setup`

## Example Output

When a whale is detected:

```
[ALERT] LARGE TRANSACTION DETECTED - Polymarket
======================================================================
Market:   Will Trump win the 2024 Presidential Election?
Outcome:  Yes
Value:    $45,250.00
Price:    $0.7500 (75.0%)
Size:     60333.33 contracts
Side:     BUY
Time:     2026-01-08T21:30:00Z

[ANOMALY INDICATORS]
  - High conviction in likely outcome

Asset ID: 65396714035221124737...
======================================================================
```

## Pro Tips

1. **Lower thresholds** for more alerts: `-t 10000`
2. **Slower polling** to reduce API calls: `-i 30`
3. **Install system-wide**: `cargo install --path .`
4. **Run in background**: `nohup whale-watcher watch > whales.log 2>&1 &`
5. **Anomaly detection**: Automatically identifies unusual trading patterns

## Troubleshooting

**Q: I get rate limit errors**  
A: Increase the interval: `whale-watcher watch -i 60`

**Q: No whales detected**  
A: Markets might be quiet. Try lowering threshold: `-t 10000`

**Q: API errors**  
A: Both APIs are public and should work. Check your internet connection.

## Next Steps

- Read the full [README.md](README.md) for detailed documentation
- Explore command options: `whale-watcher watch --help`
- Check configuration: `whale-watcher status`

Happy whale watching!
