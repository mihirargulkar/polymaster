#pragma once

#include "arbi/common.hpp"
#include "arbi/dependency_graph.hpp"
#include "arbi/kalshi_market_feed.hpp"
#include "arbi/market_feed.hpp"
#include <vector>

namespace arbi {

struct CrossExchangeResult {
  std::string timestamp;
  std::string poly_id;
  std::string kalshi_id;
  std::string action; // "BUY_POLY_YES_BUY_KALSHI_NO" etc.
  double spread;
  double net_profit;
  std::string status;
};

class CrossExchangeExecution {
public:
  CrossExchangeExecution(MarketFeed &poly_feed, KalshiMarketFeed &kalshi_feed,
                         DependencyGraph &dep_graph, const Config &config);

  // Main entry point: process matched pairs and execute profitable ones
  std::vector<CrossExchangeResult>
  process(const std::vector<CrossExchangePair> &pairs,
          const std::vector<Market> &poly_markets,
          const std::vector<Market> &kalshi_markets);

private:
  MarketFeed &poly_feed_;
  KalshiMarketFeed &kalshi_feed_;
  DependencyGraph &dep_graph_;
  Config config_;
  double current_exposure_usd_ = 0.0;

  // Execute a single arbitrage opportunity
  CrossExchangeResult executeArb(const CrossExchangePair &pair,
                                 const Market &poly_mkt,
                                 const Market &kalshi_mkt);
};

} // namespace arbi
