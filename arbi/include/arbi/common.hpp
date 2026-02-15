#pragma once
#include <Eigen/Dense>
#include <chrono>
#include <optional>
#include <string>
#include <unordered_map>
#include <vector>

namespace arbi {

// ── Configuration ────────────────────────────────────────────────────
struct Config {
  bool live_mode = false; // paper by default
  double max_trade_usd = 100.0;
  double max_exposure_usd = 500.0; // Limit total open positions
  int max_markets = 200;           // default market window
  double fee_rate = 0.02;          // Polymarket 2% on wins
  int scan_interval_s = 1;         // fast scan (1s)
  int fw_max_iters = 150;
  double fw_tolerance = 1e-8;
  double min_profit_usd = 0.50; // minimum profit to execute
  int latency_budget_ms = 2040;
  std::string groq_api_key;
  std::string groq_model = "llama-3.3-70b-versatile";
  std::string polymarket_api_key; // CLOB API key (for live orders)
  std::string polymarket_secret;
  std::string polymarket_passphrase;
};

// ── Exchange identifier ──────────────────────────────────────────────
enum class Exchange { POLYMARKET, KALSHI };

// ── Market Data ──────────────────────────────────────────────────────
struct Market {
  Exchange exchange = Exchange::POLYMARKET;
  std::string condition_id;
  std::string question;
  std::string slug;
  std::string kalshi_ticker; // e.g. "PRES-2026-DEM"
  std::string event_ticker;  // Kalshi event group
  std::string token_id_yes;
  std::string token_id_no;
  double yes_price = 0.0;
  double no_price = 0.0;
  double volume = 0.0;
  std::string category;
  bool active = true;
};

// ── Cross-Exchange Pair ──────────────────────────────────────────────
struct CrossExchangePair {
  size_t poly_idx;   // index into polymarket markets vector
  size_t kalshi_idx; // index into kalshi markets vector
  double similarity; // match confidence 0-1
  double poly_yes;   // polymarket YES price
  double kalshi_yes; // kalshi YES price
  double spread;     // absolute price difference
};

struct OrderBookLevel {
  double price;
  double size;
};

struct OrderBook {
  std::string token_id;
  std::vector<OrderBookLevel> bids;
  std::vector<OrderBookLevel> asks;

  double bestBid() const { return bids.empty() ? 0.0 : bids.front().price; }
  double bestAsk() const { return asks.empty() ? 1.0 : asks.front().price; }
  double midpoint() const { return (bestBid() + bestAsk()) / 2.0; }
  double spread() const { return bestAsk() - bestBid(); }
};

// ── Dependency Graph ─────────────────────────────────────────────────
enum class Relation {
  IMPLIES,     // x_j => x_i
  MUTEX,       // x_i + x_j <= 1
  EXACTLY_ONE, // Σ x_i = 1
  INDEPENDENT
};

struct Dependency {
  size_t market_i;
  size_t market_j;
  Relation relation;
};

// ── Arbitrage ────────────────────────────────────────────────────────
struct ArbitrageOpportunity {
  std::vector<size_t> market_indices;
  Eigen::VectorXd current_prices;
  Eigen::VectorXd optimal_prices; // projected
  Eigen::VectorXd trade_vector;   // optimal - current
  double expected_profit;
  double mispricing_pct;
  std::chrono::steady_clock::time_point detected_at;
};

// ── Execution ────────────────────────────────────────────────────────
enum class Side { BUY, SELL };

struct Order {
  std::string token_id;
  Side side;
  double price;
  double size;
  std::string order_id; // filled after submission
};

struct TradeResult {
  std::string opportunity_id;
  std::vector<Order> orders;
  double expected_pnl;
  double actual_pnl;
  double total_fees;
  double slippage;
  bool fully_filled;
  std::string status; // "FILLED", "PARTIAL", "FAILED"
  std::chrono::steady_clock::time_point executed_at;
};

// ── Timing helper ────────────────────────────────────────────────────
inline double elapsed_ms(std::chrono::steady_clock::time_point start) {
  auto now = std::chrono::steady_clock::now();
  return std::chrono::duration<double, std::milli>(now - start).count();
}

} // namespace arbi
