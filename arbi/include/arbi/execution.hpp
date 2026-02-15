#pragma once
#include "arbi/common.hpp"
#include "arbi/market_feed.hpp"
#include <vector>

namespace arbi {

class ExecutionEngine {
public:
  explicit ExecutionEngine(MarketFeed &feed, const Config &config);

  // Compute VWAP for a given trade size against the order book
  static double computeVWAP(const OrderBook &book, Side side, double size);

  // Estimate slippage at a given trade size
  static double estimateSlippage(const OrderBook &book, Side side, double size);

  // Check if trade is profitable after fees + slippage
  bool isProfitableAfterCosts(const ArbitrageOpportunity &opp,
                              const std::vector<OrderBook> &books);

  // Execute an arbitrage opportunity (all legs)
  TradeResult execute(const ArbitrageOpportunity &opp,
                      const std::vector<Market> &markets);

private:
  MarketFeed &feed_;
  Config config_;
};

} // namespace arbi
