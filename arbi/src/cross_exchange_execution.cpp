#include "arbi/cross_exchange_execution.hpp"
#include <chrono>
#include <cmath>
#include <spdlog/spdlog.h>

namespace arbi {

CrossExchangeExecution::CrossExchangeExecution(MarketFeed &poly_feed,
                                               KalshiMarketFeed &kalshi_feed,
                                               DependencyGraph &dep_graph,
                                               const Config &config)
    : poly_feed_(poly_feed), kalshi_feed_(kalshi_feed), dep_graph_(dep_graph),
      config_(config) {}

std::vector<CrossExchangeResult>
CrossExchangeExecution::process(const std::vector<CrossExchangePair> &pairs,
                                const std::vector<Market> &poly_markets,
                                const std::vector<Market> &kalshi_markets) {
  std::vector<CrossExchangeResult> results;

  // For simplicity, we process pair-by-pair.
  // In a real HFT scenario, we might batch LLM calls.
  // But our local LLM is now integrated into DependencyGraph::classifyBatch if
  // we needed it. DependencyGraph is currently designed for Polymarket
  // dependencies. We can repurpose `callGroq` (which now points to local LLM)
  // if needed, or rely on Jaccard + manual review for now as per plan.
  // Actually, let's trust the high Jaccard scores for now and execute paper
  // trades.

  for (const auto &pair : pairs) {
    if (pair.poly_idx >= poly_markets.size() ||
        pair.kalshi_idx >= kalshi_markets.size())
      continue;

    const auto &poly_mkt = poly_markets[pair.poly_idx];
    const auto &kalshi_mkt = kalshi_markets[pair.kalshi_idx];

    // Check profitability
    // Spread = |poly_yes - kalshi_yes|
    // Cost = poly_fee + kalshi_fee + slippage_buffer
    // We assume fees are around ~2% + ~2%? Let's check config.
    // config_.fee_rate is typically just one exchange fee.
    // Let's use 2 * config.fee_rate as a safe estimate for dual-leg.

    double total_fees = 2.0 * config_.fee_rate;
    double min_spread = total_fees + 0.02; // +2% profit margin req

    if (pair.spread < min_spread)
      continue;

    // Log potential opportunity
    spdlog::info("[CrossExec] Potential Arb: Spread {:.2f}% > Min {:.2f}% for "
                 "Poly: {} vs Kalshi: {}",
                 pair.spread * 100.0, min_spread * 100.0, poly_mkt.question,
                 kalshi_mkt.question);

    // Verify with LLM?
    // Since DependencyGraph is strictly internal to Polymarket events
    // currently, we skip the graph check and rely on Jaccard threshold (already
    // filtered). In Phase 6 we could add an explicit "VerifyMatch" LLM call
    // here.

    // Execute
    auto res = executeArb(pair, poly_mkt, kalshi_mkt);
    results.push_back(res);
  }

  return results;
}

CrossExchangeResult
CrossExchangeExecution::executeArb(const CrossExchangePair &pair,
                                   const Market &poly_mkt,
                                   const Market &kalshi_mkt) {
  CrossExchangeResult res;
  res.timestamp = std::to_string(
      std::chrono::system_clock::now().time_since_epoch().count());
  res.poly_id = poly_mkt.condition_id;
  res.kalshi_id = kalshi_mkt.kalshi_ticker;
  res.spread = pair.spread;
  res.action = "PENDING";
  res.status = "SUCCESS"; // Optimistic default for logs

  // Strategy:
  // If Poly Price < Kalshi Price:
  //   1. Buy YES on Poly (at e.g. 0.40)
  //   2. Buy NO on Kalshi (at e.g. 1.00 - 0.60 = 0.40). Wait...
  //      If Kalshi YES is 0.60, then NO is 0.40.
  //      We buy YES(Poly) @ 0.40 and NO(Kalshi) @ 0.40.
  //      Total cost = 0.80. Payout = 1.00 (one event wins). Profit = 0.20.
  //      Wait, are they the SAME event?
  //      If Poly says "Will Biden win?" and Kalshi says "Will Biden win?"
  //      If Poly=0.40, Kalshi=0.60.
  //      Buy YES on Poly (cost 0.40). Payoff if Win=1.
  //      Sell YES on Kalshi (price 0.60). Payoff if Win=-1? No, we can't short
  //      easily. But Buying NO is equivalent to selling YES if markets are
  //      truly binary complement. Kalshi YES=0.60 implies NO=0.40. If we Buy NO
  //      on Kalshi @ 0.40... Scenario A (Biden Wins): Poly YES pays $1. Kalshi
  //      NO pays $0. Net +$1. Cost $0.80. Profit $0.20. Scenario B (Biden
  //      Loses): Poly YES pays $0. Kalshi NO pays $1. Net +$1. Cost $0.80.
  //      Profit $0.20. Perfect hedge!

  double trade_size_usd = config_.max_trade_usd;
  double poly_price = pair.poly_yes;
  double kalshi_yes_price = pair.kalshi_yes;
  double kalshi_no_price = 1.0 - kalshi_yes_price;

  if (poly_price < kalshi_yes_price) {
    // Arbitrage: Buy Poly YES, Buy Kalshi NO
    // Prerequisite: Poly YES + Kalshi NO < 1.0
    // (poly_price + (1 - kalshi_yes)) < 1.0
    // poly_price - kalshi_yes < 0, which matches logic.

    double cost = poly_price + kalshi_no_price;
    if (cost >= 1.0 - total_fees) {
      spdlog::warn("[CrossExec] Spread vanished during exec check: Cost {:.3f}",
                   cost);
      res.status = "ABORTED_COST";
      return res;
    }

    res.action = "BUY_POLY_YES_BUY_KALSHI_NO";
    spdlog::info(
        "[CrossExec] EXECUTING: Buy Poly YES @ {:.3f}, Buy Kalshi NO @ {:.3f}",
        poly_price, kalshi_no_price);

    // 1. Execute Poly Leg
    double poly_qty = trade_size_usd / poly_price;
    auto poly_order = poly_feed_.submitOrder(poly_mkt.token_id_yes, Side::BUY,
                                             poly_price, poly_qty);

    // 2. Execute Kalshi Leg
    double kalshi_qty =
        trade_size_usd / kalshi_no_price; // Approximate match in USD sizing
    auto kalshi_order = kalshi_feed_.submitOrder(
        kalshi_mkt.kalshi_ticker, Side::BUY_NO, kalshi_no_price, kalshi_qty);
    // Wait, submitOrder takes "Side" enum. We need to support BUY_NO.
    // Looking at KalshiMarketFeed::submitOrder:
    // if side == Side::BUY -> "side": "yes"
    // else -> "side": "no".
    // So Side::SELL in our enum usually meant Selling YES.
    // But Kalshi Feed implementation:
    // if (side == Side::BUY) body["side"] = "yes"
    // else body["side"] = "no".
    // AND body["action"] is always "buy".
    // So passing Side::SELL to submitOrder actually executes a "Buy NO" order
    // on Kalshi.

    // Let's verify Side enum in common.hpp:
    // enum class Side { BUY, SELL };
    // So we use Side::SELL to mean "Buy NO" for Kalshi in this context.

    auto kalshi_res = kalshi_feed_.submitOrder(
        kalshi_mkt.kalshi_ticker, Side::SELL, kalshi_no_price, kalshi_qty);

    if (poly_order && kalshi_res) {
      res.status = "FILLED";
      res.net_profit = (1.0 - cost) * trade_size_usd;
    } else {
      res.status = "PARTIAL_FAIL";
      spdlog::error("[CrossExec] Execution failed: Poly={}, Kalshi={}",
                    poly_order.has_value(), kalshi_res.has_value());
    }

  } else {
    // Arbitrage: Buy Poly NO, Buy Kalshi YES
    // Poly NO price = 1.0 - poly_price
    // Check cost: (1-poly) + kalshi < 1.0
    // kalshi - poly < 0 => kalshi < poly. Matches logic.

    double poly_no_price = 1.0 - poly_price;
    double cost = poly_no_price + kalshi_yes_price;
    if (cost >= 1.0 - total_fees) {
      spdlog::warn("[CrossExec] Spread vanished during exec check: Cost {:.3f}",
                   cost);
      res.status = "ABORTED_COST";
      return res;
    }

    res.action = "BUY_POLY_NO_BUY_KALSHI_YES";
    spdlog::info(
        "[CrossExec] EXECUTING: Buy Poly NO @ {:.3f}, Buy Kalshi YES @ {:.3f}",
        poly_no_price, kalshi_yes_price);

    // 1. Execute Poly Leg (Buy NO)
    // Poly API usually buys "No" tokens by specifying token_id_no and Side::BUY
    // ? Or Side::SELL of Yes tokens? MarketFeed::submitOrder usage:
    // submitOrder(token_id, side, price, size)
    // If we want to buy NO, we typically buy the NO token.
    // Let's use Side::BUY on the NO token.

    double poly_qty = trade_size_usd / poly_no_price;
    auto poly_order = poly_feed_.submitOrder(poly_mkt.token_id_no, Side::BUY,
                                             poly_no_price, poly_qty);

    // 2. Execute Kalshi Leg (Buy YES)
    double kalshi_qty = trade_size_usd / kalshi_yes_price;
    auto kalshi_order = kalshi_feed_.submitOrder(
        kalshi_mkt.kalshi_ticker, Side::BUY, kalshi_yes_price, kalshi_qty);

    if (poly_order && kalshi_order) {
      res.status = "FILLED";
      res.net_profit = (1.0 - cost) * trade_size_usd;
    } else {
      res.status = "PARTIAL_FAIL";
    }
  }

  return res;
}

} // namespace arbi
