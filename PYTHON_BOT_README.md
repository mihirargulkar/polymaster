# Polymaster Python Bot

A Python-based copy-trading bot that monitors Polymarket whale activity and executes mirrored trades on Kalshi using semantic mapping (Ollama).

## 🚀 Quick Start (Recommended)

Use the provided launch script to start all services (Ollama, Dashboard, Bot):

```bash
./launch.sh
```

- **Dashboard**: [http://localhost:3000](http://localhost:3000)
- **Logs**:
    - Bot output is shown in the terminal.
    - Dashboard logs: `dashboard.log`
    - Ollama logs: `ollama.log`

## ⚙️ Configuration

Edit `python_bot/.env` to configure settings:

```env
# Mode
DRY_RUN=true              # Set to false for LIVE TRADING
ENV=development

# Trading Logic
MIN_WHALE_TRADE_USD=20000 # Minimum whale trade size to copy
FIXED_BET_USD=5.00        # Bet size on Kalshi
MIN_WIN_RATE=0.85         # Minimum whale win rate (optional filter)

# Credentials
KALSHI_KEY_ID=...
KALSHI_PRIVATE_KEY_PATH=...

# Services
OLLAMA_URL=http://localhost:11434
OLLAMA_MODEL=qwen2.5-coder:7b
POLY_WS_URL=wss://ws-subscriptions-clob.polymarket.com/ws/market
```

## 🏗️ Architecture

1.  **Polymarket Monitor** (`monitors/polymarket.py`): Listens to WebSocket for large trades.
2.  **Market Mapper** (`utils/market_mapper.py`): Uses local LLM (Ollama) to match Polymarket event titles to active Kalshi markets.
3.  **Kalshi Executor** (`executors/kalshi.py`): Places orders on Kalshi via API v2.
4.  **Dashboard** (`dashboard/`): React/Node app for visualizing alerts and trades.

## 📝 Manual Startup

If you prefer running services individually (e.g., for debugging):

**Terminal 1: Ollama**
```bash
ollama serve
```

**Terminal 2: Dashboard**
```bash
node dashboard/server.js
```

**Terminal 3: Python Bot**
```bash
export PYTHONPATH=$PYTHONPATH:$(pwd)
python3 python_bot/main.py
```
