# Polymaster

A high-conviction monitoring and visualization platform for Polymarket and Kalshi. Polymaster combines low-latency Rust detection with a real-time web dashboard for tracking whale activity.

## 🏗️ Architecture

Polymaster consists of two primary components:

1.  **Whale Watcher (Rust)**: The core engine that monitors Polymarket and Kalshi for large trades, anomalies, and repeat actors. Handles high-speed WebSocket connections and SQLite persistence.
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
- **CLI**: `wwatcher watch`, `wwatcher history`, `wwatcher setup`.
- See [docs/README_RUST.md](docs/README_RUST.md) (or use `wwatcher --help`) for detailed CLI usage.

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

---

## 📝 Documentation

- [QUICKSTART.md](QUICKSTART.md) — Comprehensive setup guide.
- [docs/API_REFERENCE.md](docs/API_REFERENCE.md) — Documentation for 15+ used endpoints.
- [docs/WEBHOOK_REFERENCE.md](docs/WEBHOOK_REFERENCE.md) — Webhook schema for third-party integrations.

## ⚖️ License & Disclaimer
This tool is for informational and research purposes only. Use this data solely for informed decision-making and market analysis.
