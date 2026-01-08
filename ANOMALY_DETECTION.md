# Anomaly Detection

Whale Watcher includes built-in anomaly detection to identify unusual trading patterns that may indicate significant market events, information asymmetry, or non-standard trading behavior.

## Detection Categories

### 1. Extreme Confidence Bets

**Trigger**: Price > 95% or < 5%

Identifies trades on outcomes with very high or very low implied probabilities. These can indicate:
- Strong conviction in near-certain outcomes
- Potential arbitrage opportunities
- Market inefficiencies
- Contrarian positions betting against consensus

**Example**:
```
Price: $0.98 (98.0%)
[ANOMALY INDICATORS]
  - Extreme confidence bet (98.0% probability)
```

### 2. Contrarian Positions

**Trigger**: Price < 5%

Large bets on highly unlikely outcomes. These are particularly interesting because they may signal:
- Insider information
- Hedging strategies
- Mispriced markets
- High-risk, high-reward plays

**Example**:
```
Price: $0.03 (3.0%)
Value: $75,000.00
[ANOMALY INDICATORS]
  - Contrarian position (3.0% probability)
  - Significant bet on unlikely outcome - possible hedge or information asymmetry
```

### 3. Exceptionally Large Position Sizes

**Trigger**: Size > 100,000 contracts

Identifies unusually large position sizes that exceed typical market activity. These can indicate:
- Institutional trading
- Major conviction trades
- Portfolio rebalancing
- Market manipulation attempts

**Example**:
```
Size: 125,000.00 contracts
[ANOMALY INDICATORS]
  - Exceptionally large position size
```

### 4. Major Capital Deployment

**Trigger**: Transaction value > $100,000

Flags transactions with very high dollar values. These represent:
- Significant capital at risk
- Professional or institutional trading
- Major market events
- Whale activity

**Example**:
```
Value: $150,000.00
[ANOMALY INDICATORS]
  - Major capital deployment: $150000
```

### 5. High Conviction in Likely Outcomes

**Trigger**: Price > 90% AND Size > 50,000 contracts

Large bets on near-certain outcomes. While lower risk, the large size indicates:
- Capital efficiency plays
- Portfolio strategies
- Pre-resolution positioning
- Arbitrage opportunities

**Example**:
```
Price: $0.92 (92.0%)
Size: 75,000.00 contracts
[ANOMALY INDICATORS]
  - High conviction in likely outcome
```

### 6. Information Asymmetry Indicators

**Trigger**: Price < 20% AND Value > $50,000

Large bets on unlikely outcomes often signal:
- Potential insider information
- Sophisticated hedging strategies
- Mispriced markets
- Non-public information

**Example**:
```
Price: $0.15 (15.0%)
Value: $60,000.00
[ANOMALY INDICATORS]
  - Significant bet on unlikely outcome - possible hedge or information asymmetry
```

## Multiple Anomalies

Transactions can trigger multiple anomaly indicators simultaneously, providing richer context:

```
[ALERT] LARGE TRANSACTION DETECTED - Polymarket
======================================================================
Market:   Will candidate X win state Y?
Outcome:  Yes
Value:    $125,000.00
Price:    $0.97 (97.0%)
Size:     128,865.98 contracts
Side:     BUY
Time:     2026-01-08T21:45:00Z

[ANOMALY INDICATORS]
  - Extreme confidence bet (97.0% probability)
  - Exceptionally large position size
  - Major capital deployment: $125000
  - High conviction in likely outcome

Asset ID: 65396714035221124737...
======================================================================
```

## Use Cases

### Market Research
- Identify consensus changes in real-time
- Track institutional money flow
- Discover mispriced markets

### Risk Analysis
- Monitor extreme position concentrations
- Detect potential manipulation
- Track large counterparty exposure

### Trading Signals
- Follow smart money
- Identify arbitrage opportunities
- Spot market inefficiencies

### Investigation
- Detect potential insider trading
- Track suspicious patterns
- Monitor market manipulation

## Technical Implementation

The anomaly detection system runs automatically on every transaction that exceeds your threshold. It:

1. Analyzes price probability
2. Evaluates position size
3. Calculates transaction value
4. Checks for pattern combinations
5. Displays relevant indicators

No additional configuration is required - anomaly detection is always active when watching for whales.

## Interpretation Guidelines

### High Confidence Bets (>90%)
- Lower risk, but large size still significant
- May indicate pre-resolution positioning
- Often used for capital efficiency

### Low Probability Bets (<20%)
- Higher risk profiles
- Could signal special information
- Frequently used in hedging strategies

### Large Sizes (>100k contracts)
- Institutional-level activity
- Major market participant
- Significant conviction or strategy

### High Values (>$100k)
- Professional traders
- Serious capital deployment
- Worth detailed analysis

## Limitations

Anomaly detection is heuristic-based and should be used as an investigative tool, not definitive proof of any particular behavior. Consider:

- Market context and timing
- Overall market liquidity
- Historical trading patterns
- External events and news

Always conduct additional research when investigating flagged transactions.

## Future Enhancements

Potential improvements to the anomaly detection system:

- Statistical modeling of "normal" trading patterns
- Machine learning for pattern recognition
- Historical baseline comparisons
- Cross-market correlation analysis
- Time-series anomaly detection
- Network analysis of related trades
