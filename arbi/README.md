# Arbi — Polymarket Mathematical Arbitrage Bot

High-performance C++ arbitrage detection and execution engine for Polymarket prediction markets.

## Architecture

```
Markets → Dependency Graph → Marginal Polytope → Frank-Wolfe → Execution
  (API)     (Groq LLM)        (GLPK IP)         (Optimizer)    (CLOB)
```

### 5-Layer Pipeline

| Layer | Component | Description |
|-------|-----------|-------------|
| 1 | **Market Feed** | REST client for Polymarket CLOB API |
| 2 | **Dependency Graph** | Groq LLM classifies market relationships |
| 3 | **Marginal Polytope** | IP constraints via GLPK for arbitrage detection |
| 4 | **Bregman + Frank-Wolfe** | KL-divergence projection for optimal trades |
| 5 | **Execution Engine** | VWAP + slippage-aware order submission |

## Quick Start

```bash
# Paper mode (default)
./run_arbi.sh

# Live mode
./run_arbi.sh --live --max-trade 50

# Custom scan interval
./run_arbi.sh --scan-interval 60 --min-profit 1.0
```

## Build Manually

```bash
mkdir -p build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
make -j$(sysctl -n hw.ncpu)
```

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `GROQ_API_KEY` | ✅ | Groq API key for dependency discovery |
| `POLY_API_KEY` | Live only | Polymarket CLOB API key |
| `POLY_API_SECRET` | Live only | Polymarket CLOB API secret |
| `POLY_PASSPHRASE` | Live only | Polymarket CLOB passphrase |

## CLI Options

```
--live              Live trading mode
--paper             Paper trading (default)
--max-trade <USD>   Max trade size (default: 100)
--scan-interval <S> Scan interval seconds (default: 30)
--min-profit <USD>  Min profit to execute (default: 0.50)
--fw-iters <N>      Frank-Wolfe iterations (default: 150)
```

## Output

- `logs/trades.csv` — All executed trades with P&L
- `logs/opportunities.csv` — All detected arbitrage opportunities

## Dependencies

Built automatically via CMake FetchContent:
- **nlohmann/json** — JSON parsing
- **Eigen** — Linear algebra
- **spdlog** — Structured logging
- **IXWebSocket** — WebSocket client
- **GLPK** — LP/IP solver (system: `brew install glpk`)
- **libcurl** — HTTP client (system)
