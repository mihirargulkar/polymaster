#include "arbi/cross_exchange_execution.hpp"
#include "arbi/execution.hpp"
#include <chrono>
#include <cmath>
#include <spdlog/spdlog.h>

namespace arbi {

CrossExchangeExecution::CrossExchangeExecution(MarketFeed &poly_feed,
                                               KalshiMarketFeed &kalshi_feed,
                                               DependencyGraph &dep_graph,
                                               const Config &config)
    : poly_feed_(poly_feed), kalshi_feed_(kalshi_feed), dep_graph_(dep_graph),
      config_(config), current_exposure_usd_(0.0) {}

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
  res.status = "SUCCESS";

  double trade_size_usd = config_.max_trade_usd;
  double total_fees = 2.0 * config_.fee_rate;

  // ── Global Exposure Check ──
  if (current_exposure_usd_ + trade_size_usd > config_.max_exposure_usd) {
    spdlog::warn(
        "[CrossExec] exposure limit reached ({:.2f} + {:.2f} > {:.2f})",
        current_exposure_usd_, trade_size_usd, config_.max_exposure_usd);
    res.status = "ABORTED_EXPOSURE";
    return res;
  }

  double poly_price = pair.poly_yes;
  double kalshi_yes_price = pair.kalshi_yes;

  // ── Pre-trade VWAP & Slippage Check ──
  std::string poly_tid_buy;
  std::string kalshi_ticker = kalshi_mkt.kalshi_ticker;
  Side poly_side = Side::BUY;
  Side kalshi_side = Side::BUY;

  bool buy_poly_yes = (poly_price < kalshi_yes_price);
  if (buy_poly_yes) {
    poly_tid_buy = poly_mkt.token_id_yes;
    kalshi_side = Side::SELL; // Buy NO
  } else {
    poly_tid_buy = poly_mkt.token_id_no;
    kalshi_side = Side::BUY; // Buy YES
  }

  // Fetch Books
  auto poly_book = poly_feed_.fetchOrderBook(poly_tid_buy);
  auto kalshi_book = kalshi_feed_.fetchOrderBook(kalshi_ticker);

  // Compute VWAP
  double poly_vwap =
      ExecutionEngine::computeVWAP(poly_book, Side::BUY, trade_size_usd);

  double kalshi_vwap = 0.0;
  if (kalshi_side == Side::BUY) {
    kalshi_vwap =
        ExecutionEngine::computeVWAP(kalshi_book, Side::BUY, trade_size_usd);
  } else {
    // Selling YES = Buying NO. Check YES Bids.
    kalshi_vwap =
        ExecutionEngine::computeVWAP(kalshi_book, Side::SELL, trade_size_usd);
  }

  if (poly_vwap < 1e-6 || kalshi_vwap < 1e-6) {
    spdlog::warn(
        "[CrossExec] Low liquidity. PolyVWAP={:.3f}, KalshiVWAP={:.3f}",
        poly_vwap, kalshi_vwap);
    res.status = "ABORTED_LIQUIDITY";
    return res;
  }

  // Re-evaluate cost
  double real_cost = 0.0;
  if (buy_poly_yes) {
    // Buy Poly YES + Buy Kalshi NO
    // Kalshi VWAP is YES Bid price.
    // Cost of NO = 1.0 - YES Bid.
    real_cost = poly_vwap + (1.0 - kalshi_vwap);
  } else {
    // Buy Poly NO + Buy Kalshi YES
    real_cost = poly_vwap + kalshi_vwap;
  }

  if (real_cost >= 1.0 - total_fees) {
    spdlog::warn("[CrossExec] VWAP Spread too thin. Cost {:.3f}", real_cost);
    res.status = "ABORTED_SLIPPAGE";
    return res;
  }

  // Execute
  if (buy_poly_yes) {
    res.action = "BUY_POLY_YES_BUY_KALSHI_NO";
    spdlog::info(
        "[CrossExec] EXEC: Buy Poly YES @ {:.3f}, Buy Kalshi NO @ {:.3f}",
        poly_vwap, 1.0 - kalshi_vwap);

    // Poly
    double poly_qty = trade_size_usd / poly_vwap;
    auto p_ord =
        poly_feed_.submitOrder(poly_tid_buy, Side::BUY, poly_vwap, poly_qty);

    // Kalshi (Buy NO) -> price is (1 - kalshi_vwap)
    double k_price = 1.0 - kalshi_vwap;
    double k_qty = trade_size_usd / k_price;
    auto k_ord =
        kalshi_feed_.submitOrder(kalshi_ticker, Side::SELL, k_price, k_qty);

    if (p_ord && k_ord) {
      res.status = "FILLED";
      res.net_profit = (1.0 - real_cost) * trade_size_usd;
      current_exposure_usd_ += trade_size_usd;
    } else {
      res.status = "PARTIAL_FAIL";
    }

  } else {
    res.action = "BUY_POLY_NO_BUY_KALSHI_YES";
    spdlog::info(
        "[CrossExec] EXEC: Buy Poly NO @ {:.3f}, Buy Kalshi YES @ {:.3f}",
        poly_vwap, kalshi_vwap);

    double poly_qty = trade_size_usd / poly_vwap;
    auto p_ord =
        poly_feed_.submitOrder(poly_tid_buy, Side::BUY, poly_vwap, poly_qty);

    double k_qty = trade_size_usd / kalshi_vwap;
    auto k_ord =
        kalshi_feed_.submitOrder(kalshi_ticker, Side::BUY, kalshi_vwap, k_qty);

    if (p_ord && k_ord) {
      res.status = "FILLED";
      res.net_profit = (1.0 - real_cost) * trade_size_usd;
      current_exposure_usd_ += trade_size_usd;
    } else {
      res.status = "PARTIAL_FAIL";
    }
  }

  return res;
}

} // namespace arbi
