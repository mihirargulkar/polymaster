# Polymaster

A high-conviction monitoring and visualization platform for Polymarket and Kalshi. Polymaster combines low-latency Rust detection with a real-time web dashboard for tracking whale activity. **LLM-powered market matching** maps Polymarket whale signals to Kalshi markets and can **auto-trade** those signals on Kalshi when configured.

## 🏗️ Architecture

Polymaster consists of two primary components:

1.  **Whale Watcher (Rust)**: The core engine that monitors Polymarket and Kalshi for large trades, anomalies, and repeat actors. Handles high-speed WebSocket connections and SQLite persistence.
    - **LLM matching**: Uses a local LLM (Ollama) to match Polymarket event titles to active Kalshi markets so whale signals can be mirrored.
    - **Auto-trading**: When Kalshi API credentials are set, the watcher automatically places limit orders on Kalshi for matched Polymarket whale alerts (signals from the whale logs / live stream).
2.  **Dashboard (Node.js)**: A React/Express-based real-time visualizer for alerts and market activity.

---

## 🚀 Quick Start

Use the unified launch script to start the monitoring engine and the dashboard:

```bash
# 1. Clone the repository
git clone https://github.com/neur0map/polymaster.git
cd polymaster

# 2. Configure environment
cp .env.example .env 

# 3. Launch system
./launch.sh
```

- **Dashboard**: [http://localhost:3000](http://localhost:3000)
- **Logs**:
    - Rust Watcher: `watcher_rust.log`
    - Dashboard: `dashboard.log`

---

## 🐋 Components

### 1. Whale Watcher (Rust)
Real-time monitoring of Polymarket and Kalshi transactions with anomaly detection.
- **Key Features**: WebSocket support, 12-hour wallet memory, top holder analysis, and configurable thresholds.
- **LLM matching (Polymarket → Kalshi)**: Each Polymarket whale alert is sent to Ollama with a list of active Kalshi markets; the LLM returns a matching ticker and side (yes/no) when confidence &gt; 0.8. Requires [Ollama](https://ollama.ai) running locally (e.g. `ollama serve` and `ollama pull llama3` or `qwen2.5-coder:7b`).
- **Auto-trading**: Whale alerts are logged to the DB; when a Polymarket alert matches a Kalshi market, the watcher can place a limit order on Kalshi (configurable bet size). Signals are taken from the **live whale stream** and persisted in the same DB the dashboard reads (whale logs).
- **CLI**: `wwatcher watch`, `wwatcher history`, `wwatcher setup`.
- Run `wwatcher --help` for detailed CLI usage.

### 2. Dashboard (Node.js)
Visualize whale alerts and market sentiment as they happen.
- **Real-time updates**: Pushes alerts directly to the web interface.
- **Visuals**: Charts alerting frequency and transaction size.
- **Start manually**: `node dashboard/server.js`

---

## ⚙️ Configuration

1.  **.env (Root)**: Basic environment setup.
2.  **config.json (`~/.config/wwatcher/`)**: Used by the Rust component for platform selection and thresholds.
    - Run `wwatcher setup` for a guided wizard.
    - **LLM matching**: Set `ollama_model` (e.g. `"llama3"` or `"qwen2.5-coder:7b"`) and optionally `ollama_url` (default `"http://localhost:11434"`). Ensure Ollama is running and the model is pulled.
    - **Auto-trading**: Set `kalshi_api_key_id` and `kalshi_private_key` (path to PEM or PEM string) and optionally `bet_size` (USD per matched signal). Use `kalshi_is_demo: true` for the demo exchange.

---

## ⚖️ License & Disclaimer
This tool is for informational and research purposes only. Use this data solely for informed decision-making and market analysis.
