#include "arbi/common.hpp"
#include "arbi/cross_exchange_execution.hpp"
#include "arbi/dependency_graph.hpp"
#include "arbi/execution.hpp"
#include "arbi/frank_wolfe.hpp"
#include "arbi/kalshi_feed.hpp"
#include "arbi/kalshi_market_feed.hpp"
#include "arbi/logger.hpp"
#include "arbi/market_feed.hpp"
#include "arbi/polytope.hpp"
#include "arbi/websocket_feed.hpp"

#include <Eigen/Dense>
#include <spdlog/sinks/stdout_color_sinks.h>
#include <spdlog/spdlog.h>

#include <csignal>
#include <cstdlib>
#include <iostream>
#include <string>
#include <thread>

using namespace arbi;

static volatile std::sig_atomic_t running = 1;

static void signalHandler(int) { running = 0; }

// ── Parse CLI args ──────────────────────────────────────────────────
static Config parseArgs(int argc, char *argv[]) {
  Config cfg;

  // Load from environment
  if (auto *v = std::getenv("GROQ_API_KEY"))
    cfg.groq_api_key = v;
  if (auto *v = std::getenv("POLY_API_KEY"))
    cfg.polymarket_api_key = v;
  if (auto *v = std::getenv("POLY_API_SECRET"))
    cfg.polymarket_secret = v;
  if (auto *v = std::getenv("POLY_PASSPHRASE"))
    cfg.polymarket_passphrase = v;

  for (int i = 1; i < argc; i++) {
    std::string arg = argv[i];

    if (arg == "--live")
      cfg.live_mode = true;
    else if (arg == "--paper")
      cfg.live_mode = false;
    else if (arg == "--max-trade" && i + 1 < argc)
      cfg.max_trade_usd = std::stod(argv[++i]);
    else if (arg == "--limit" && i + 1 < argc)
      cfg.max_markets = std::stoi(argv[++i]);
    else if (arg == "--scan-interval" && i + 1 < argc)
      cfg.scan_interval_s = std::stoi(argv[++i]);
    else if (arg == "--min-profit" && i + 1 < argc)
      cfg.min_profit_usd = std::stod(argv[++i]);
    else if (arg == "--fw-iters" && i + 1 < argc)
      cfg.fw_max_iters = std::stoi(argv[++i]);
    else if (arg == "--help" || arg == "-h") {
      std::cout << R"(
╔═══════════════════════════════════════════════════════════╗
║              ARBI — Polymarket Arbitrage Bot              ║
║         Marginal Polytope · Bregman · Frank-Wolfe         ║
╚═══════════════════════════════════════════════════════════╝

Usage: arbi [OPTIONS]

Options:
  --live                Enable live order execution (default: paper)
  --paper               Paper trading mode (default)
  --max-trade <USD>     Maximum trade size in USD (default: 100)
  --scan-interval <SEC> Scan interval in seconds (default: 1)
  --limit <N>           Max markets to scan (default: 200)
  --min-profit <USD>    Minimum profit to execute (default: 0.50)
  --fw-iters <N>        Frank-Wolfe max iterations (default: 150)
  --help, -h            Show this help

Environment:
  GROQ_API_KEY          Groq API key for dependency discovery
  POLY_API_KEY          Polymarket CLOB API key (live mode)
  POLY_API_SECRET       Polymarket CLOB API secret (live mode)
  POLY_PASSPHRASE       Polymarket CLOB passphrase (live mode)
)";
      std::exit(0);
    }
  }
  return cfg;
}

// ── Main pipeline ────────────────────────────────────────────────────
int main(int argc, char *argv[]) {
  // Setup logging
  auto console = spdlog::stdout_color_mt("arbi");
  spdlog::set_default_logger(console);
  spdlog::set_level(spdlog::level::info);
  spdlog::set_pattern("[%H:%M:%S.%e] [%^%l%$] %v");

  // Parse config
  Config cfg = parseArgs(argc, argv);

  // Signal handler for graceful shutdown
  std::signal(SIGINT, signalHandler);
  std::signal(SIGTERM, signalHandler);

  // Banner
  spdlog::info("╔═══════════════════════════════════════════════════════╗");
  spdlog::info("║              ARBI — Polymarket Arbitrage Bot          ║");
  spdlog::info("╚═══════════════════════════════════════════════════════╝");
  spdlog::info("Mode: {}", cfg.live_mode ? "🔴 LIVE" : "📝 PAPER");
  spdlog::info("Max trade: ${:.2f}", cfg.max_trade_usd);
  spdlog::info("Scan interval: {}s", cfg.scan_interval_s);
  spdlog::info("Min profit: ${:.2f}", cfg.min_profit_usd);
  spdlog::info("FW iterations: {}", cfg.fw_max_iters);
  spdlog::info("Groq API: {}", cfg.groq_api_key.empty() ? "❌ missing" : "✅");

  if (cfg.groq_api_key.empty()) {
    spdlog::error("GROQ_API_KEY not set. Required for dependency discovery.");
    return 1;
  }

  if (cfg.live_mode && cfg.polymarket_api_key.empty()) {
    spdlog::error("POLY_API_KEY not set. Required for live trading.");
    return 1;
  }

  // ── Initialize components ────────────────────────────────────────
  MarketFeed feed(cfg);
  DependencyGraph depGraph(cfg);
  MarginalPolytope polytope;
  FrankWolfe fw;
  ExecutionEngine exec(feed, cfg);
  CrossExchangeExecution crossExec(feed, *kalshiMarketFeed, depGraph, cfg);
  Logger logger("logs");

  // ── WebSocket Integration ────────────────────────────────────────
  WebSocketFeed wsFeed;
  KalshiFeed kalshiFeed;
  std::mutex price_mutex;
  std::unordered_map<std::string, double> price_cache;

  wsFeed.setOnUpdate([&](const OrderBookUpdate &update) {
    std::lock_guard<std::mutex> lock(price_mutex);
    price_cache[update.token_id] = update.price;
    // Also maybe store volume/size? For now just price.
  });

  // Get Kalshi keys from env
  const char *k_key_id = std::getenv("KALSHI_API_KEY_ID");
  const char *k_priv_key = std::getenv("KALSHI_PRIVATE_KEY_PATH");
  std::unique_ptr<KalshiMarketFeed> kalshiMarketFeed;
  if (k_key_id && k_priv_key) {
    kalshiFeed.setup(k_key_id, k_priv_key);
    kalshiFeed.connect();
    kalshiMarketFeed = std::make_unique<KalshiMarketFeed>(k_key_id, k_priv_key);
  } else {
    spdlog::warn("Kalshi credentials not found in env, skipping Kalshi feed.");
  }

  wsFeed.connect();

  // Wait for connection
  std::this_thread::sleep_for(std::chrono::seconds(2));

  int cycle = 0;
  int last_markets_scanned = 0;

  std::vector<Market> markets;
  std::vector<Market> kalshi_markets;
  std::vector<CrossExchangePair> xpairs;
  auto last_market_fetch = std::chrono::steady_clock::time_point::min();
  const auto MARKET_REFRESH_INTERVAL = std::chrono::seconds(60);

  // ── Main loop ────────────────────────────────────────────────────
  while (running) {
    cycle++;
    auto cycle_start = std::chrono::steady_clock::now();
    int opportunities_found = 0;

    try {
      // ── Step 1: Fetch/Refresh Markets (Periodic) ────────────────
      if (markets.empty() ||
          (cycle_start - last_market_fetch > MARKET_REFRESH_INTERVAL)) {
        spdlog::info("Refreshing markets list via HTTP...");
        auto new_markets = feed.fetchMarkets();
        last_market_fetch = cycle_start;

        if (new_markets.empty()) {
          spdlog::warn("No markets fetched, retrying...");
          std::this_thread::sleep_for(std::chrono::seconds(1)); // Retry soon
          continue;
        }

        markets = new_markets;
        last_markets_scanned = (int)markets.size();

        // Subscribe to new token IDs via WebSocket
        std::vector<std::string> token_ids;
        for (const auto &m : markets) {
          if (!m.token_id_yes.empty())
            token_ids.push_back(m.token_id_yes);
          if (!m.token_id_no.empty())
            token_ids.push_back(m.token_id_no);
        }
        wsFeed.subscribe(token_ids);
        spdlog::info("Subscribed to {} tokens", token_ids.size());

        // Also fetch Kalshi markets and match
        if (kalshiMarketFeed) {
          kalshi_markets = kalshiMarketFeed->fetchMarkets();
          xpairs = KalshiMarketFeed::matchMarkets(markets, kalshi_markets);
          spdlog::info("[CrossExchange] Found {} matching pairs",
                       xpairs.size());

          // Execute Cross-Exchange Arbitrage
          auto results = crossExec.process(xpairs, markets, kalshi_markets);
          for (const auto &res : results) {
            spdlog::info("[CrossExec] {} | {} ↔ {} | Net: ${:.2f} | Status: {}",
                         res.timestamp, res.poly_id, res.kalshi_id,
                         res.net_profit, res.status);
          }
        }
      }

      // ── Step 1.5: Update Prices from WS Cache ───────────────────
      {
        std::lock_guard<std::mutex> lock(price_mutex);
        for (auto &m : markets) {
          if (price_cache.count(m.token_id_yes)) {
            m.yes_price = price_cache[m.token_id_yes];
          }
          if (price_cache.count(m.token_id_no)) {
            m.no_price = price_cache[m.token_id_no];
          }
        }
      }

      // ── Step 2: Discover/Get dependencies ───────────────────
      // Launch async discovery if markets changed significantly
      // (For now, just launch it periodically if not running)
      if (cycle % 20 == 0) {
        depGraph.startAsyncDiscovery(markets);
      }

      // Get currently known dependencies (non-blocking)
      auto deps = depGraph.getDependencies(markets);

      // ── Step 3: Build marginal polytope ─────────────────────
      polytope.buildConstraints(markets.size(), deps);

      if (polytope.numConstraints() == 0) {
        // Independent markets optimization?
        //  Skip for now to focus on Arb.
        if (cycle % 100 == 0)
          spdlog::info("No constraints (cycle {}), running...", cycle);
        // std::this_thread::sleep_for(std::chrono::milliseconds(100));
        // continue;
      }

      // ── Step 4: Build price vector ──────────────────────────
      Eigen::VectorXd prices(markets.size());
      for (size_t i = 0; i < markets.size(); i++) {
        prices[i] = markets[i].yes_price;
      }

      // ── Step 5: Check feasibility ───────────────────────────
      auto feasResult = polytope.checkFeasibility(prices);

      if (feasResult.feasible) {
        // spdlog::info("Prices feasible.");
        // logger.logCycle(cycle, markets.size(), 0, elapsed_ms(cycle_start));
        std::this_thread::sleep_for(std::chrono::milliseconds(100));
        continue;
      }

      spdlog::info("⚡ Arbitrage detected! Violation={:.6f}",
                   feasResult.violation);

      // ── Step 6: Frank-Wolfe optimization ────────────────────
      auto fwResult =
          fw.optimize(prices, polytope, cfg.fw_max_iters, cfg.fw_tolerance);

      if (fwResult.profit < cfg.min_profit_usd) {
        spdlog::info("Profit ${:.4f} below minimum ${:.2f}, skipping",
                     fwResult.profit, cfg.min_profit_usd);
        logger.logCycle(cycle, markets.size(), 0, elapsed_ms(cycle_start));
        std::this_thread::sleep_for(
            std::chrono::milliseconds(100)); // Fast retry
        continue;
      }

      // ── Step 7: Construct opportunity ───────────────────────
      ArbitrageOpportunity opp;
      opp.current_prices = prices;
      opp.optimal_prices = fwResult.optimal;
      opp.trade_vector = fwResult.trade_vector;
      opp.expected_profit = fwResult.profit;
      opp.detected_at = std::chrono::steady_clock::now();

      // Find which markets have non-trivial trade vectors
      for (size_t i = 0; i < markets.size(); i++) {
        if (std::abs(fwResult.trade_vector[i]) > 1e-6) {
          opp.market_indices.push_back(i);
        }
      }
      opp.mispricing_pct = feasResult.violation;

      opportunities_found++;
      logger.logOpportunity(opp, markets);

      // ── Step 8: Get order books for involved markets ─────────
      // Use WebSocket order book cache (zero latency) with HTTP fallback
      std::vector<OrderBook> books;
      for (auto idx : opp.market_indices) {
        const auto &tid = markets[idx].token_id_yes;
        if (wsFeed.orderBookCache().has(tid)) {
          books.push_back(wsFeed.orderBookCache().get(tid));
        } else {
          // Cold start fallback: fetch via HTTP (slow, ~200ms)
          spdlog::warn("OB cache miss for {}, fetching via HTTP",
                       tid.substr(0, 12));
          books.push_back(feed.fetchOrderBook(tid));
        }
      }

      // ── Step 9: Profitability check after costs ─────────────
      if (!exec.isProfitableAfterCosts(opp, books)) {
        spdlog::info("Not profitable after fees+slippage, skipping");
        logger.logCycle(cycle, markets.size(), opportunities_found,
                        elapsed_ms(cycle_start));
        std::this_thread::sleep_for(std::chrono::milliseconds(100));
        continue;
      }

      // ── Step 10: Execute ─────────────────────────────────────
      auto tradeResult = exec.execute(opp, markets);
      logger.logTrade(tradeResult);

    } catch (const std::exception &e) {
      spdlog::error("Cycle {} error: {}", cycle, e.what());
    }

    // logger.logCycle(cycle, last_markets_scanned, opportunities_found,
    //                 elapsed_ms(cycle_start));

    // Wait before next scan (Fast)
    if (running) {
      std::this_thread::sleep_for(std::chrono::milliseconds(100)); // 100ms
    }
  }

  spdlog::info("Shutting down gracefully after {} cycles.", cycle);
  return 0;
}
