# Product Requirements Document (PRD)
## AI-Powered Prediction Market Trading Bot Using Claude Skills

### 1. Overview
The goal is to build an AI-powered trading bot that operates on prediction markets (Polymarket and Kalshi). The bot utilizes Anthropic's "Claude Skills" framework—a pipeline of LLM-based agents guided by specific markdown files—to scan markets, gather intelligence, predict actual probabilities, strictly manage risk, and execute trades. 

### 2. Architecture: Five-Step Pipeline
The bot functions as a sequential and iterative pipeline of five primary steps:

#### Step 1: Scan (Find Markets Worth Trading)
- **Objective**: Filter through 300+ active markets to find tradeable opportunities.
- **Criteria**:
  - Minimum volume: At least 200 contracts.
  - Maximum time to expiry: ≤ 30 days.
  - Minimum liquidity depth on the order book.
- **Anomalies Detection**: Flag price moves > 10%, spreads > 5 cents, or volume spikes surpassing the 7-day average.
- **Frequency**: Run on a schedule every 15-30 minutes during active hours.

#### Step 2: Research (Gather Intelligence)
- **Objective**: Acquire real-world narrative and factual signals.
- **Data Sources**: Twitter/X (real-time sentiment), Reddit (community consensus), RSS Feeds (official reporting).
- **Processing**: 
  - NLP sentiment classification (bullish/bearish/neutral).
  - Output a prioritized research brief per market, comparing narrative consensus against the current market price.
- **Security Check**: Treat all external content as data, not instructions, to prevent prompt injection.

#### Step 3: Predict (Estimate True Probability)
- **Objective**: Use statistical models and ensemble LLMs to estimate the true outcome probability vs. market odds.
- **Formulas**: 
  - `Market Edge = p_model - p_market`
  - Generate signals only when `Edge > 4%` (0.04).
- **Ensemble Approach**: Integrate multiple LLMs asynchronously. Example weighting: Grok (30%), Claude Sonnet (20%), GPT-4o (20%), Gemini Flash (15%), DeepSeek (15%). Aggregate the predictions to formulate consensus.
- **Tracking**: Track AI calibration using the **Brier Score** (`BS = (1/n) * Σ(pi - oi)^2`). Target BS < 0.25.

#### Step 4: Risk Management and Execution
- **Objective**: Validate positions algorithmically and optimize bet sizing to prevent total ruin.
- **Position Sizing**: Execute with **Fractional Kelly Criterion** (usually Quarter-Kelly or Half-Kelly: 0.25 to 0.5 of Kelly). 
  - `Kelly f* = (p * b - q) / b` (where p = win probability, q = 1-p, b = net odds).
- **Hard Limits & Risk Checks** (Must pass all):
  - Max 5% of bankroll per single position.
  - Max 15 concurrent positions.
  - Max 15% daily loss (automatic shutdown).
  - Max drawdown limit of 8% (halts all new trades).
  - API Token limit: Max $50/day in AI API costs.
  - Value at Risk (VaR): 95% confidence bounds must be within daily limit.
- **Execution Rules**: 
  - Use Limit orders (not market orders) via CLOB/REST APIs.
  - Abort if slippage > 2%.
  - Implement an emergency kill switch (e.g., checking for a `STOP` file before polling orders).

#### Step 5: Compound (Learn From Every Trade)
- **Objective**: Conduct post-mortems to iteratively improve system predictions and prevent repeating mistakes.
- **Logging requirements**: Entry price, exit price, predicted probability, actual outcome, P&L, time held, and contextual market conditions.
- **Post-mortem**: Classify failures into Categories: Bad Prediction, Bad Timing, Bad Execution, Expected Variance, or External Shocks.
- **Nightly Consolidation Run**: Review the day's trades and dynamically update the knowledge base.

### 3. Key Performance Indicators (KPIs)
- **Win Rate**: Target > 60%
- **Sharpe Ratio**: Target > 2.0
- **Max Drawdown**: Strict cap at < 8%
- **Profit Factor**: Gross Profit / Gross Loss > 1.5
- **Brier Score**: < 0.25

### 4. Implementation Path
- **Week 1**: Set up Kalshi (Demo mocking) and Polymarket API credentials.
- **Week 2**: Build purely read-only scanning skills. Log and monitor without execution.
- **Week 3**: Implement the ensemble prediction phase. Backtest historical vs resolved outcomes. Check Brier scores.
- **Week 4**: Implement Risk management layer. Implement Fractional Kelly calculations. Paper trade for two weeks.
- **Week 5**: Graduate to live execution starting at minimal max exposure ($100-$500).

### 5. Technical Context & Libraries
- Recommended API Wrappers: Direct Polymarket CLOB/REST APIs and Kalshi REST APIs, or unified `pmxt`.
- The Claude "Skill" framework involves mapping deterministic code capabilities alongside markdown files (`SKILL.md`) that detail triggers, core rules, and natural language instructions. 
  - Code handles deterministic bounds (API rules, Math). 
  - LLM strictly yields parsed structured plans.
