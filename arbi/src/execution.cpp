#include "arbi/execution.hpp"
#include <chrono>
#include <cmath>
#include <future>
#include <spdlog/spdlog.h>
#include <vector>

namespace arbi {

ExecutionEngine::ExecutionEngine(MarketFeed &feed, const Config &config)
    : feed_(feed), config_(config) {}

// ── VWAP calculation ─────────────────────────────────────────────────
// ── VWAP calculation ─────────────────────────────────────────────────
// ── VWAP calculation ─────────────────────────────────────────────────
double ExecutionEngine::computeVWAP(const OrderBook &book, Side side,
                                    double size) {
  const auto &levels = (side == Side::BUY) ? book.asks : book.bids;

  if (levels.empty())
    return 0.0;

  double remaining = size;
  double total_cost = 0.0;
  double total_filled = 0.0;

  for (auto &level : levels) {
    double fill = std::min(remaining, level.size);
    total_cost += fill * level.price;
    total_filled += fill;
    remaining -= fill;

    if (remaining <= 0)
      break;
  }

  if (total_filled == 0.0)
    return 0.0;
  return total_cost / total_filled;
}

// ── Slippage estimation ──────────────────────────────────────────────
// ── Slippage estimation ──────────────────────────────────────────────
// ── Slippage estimation ──────────────────────────────────────────────
double ExecutionEngine::estimateSlippage(const OrderBook &book, Side side,
                                         double size) {
  double vwap = computeVWAP(book, side, size);
  double best = (side == Side::BUY) ? book.bestAsk() : book.bestBid();

  if (best == 0.0)
    return 1.0; // max slippage

  return std::abs(vwap - best) / best;
}

// ── Profitability check ──────────────────────────────────────────────
bool ExecutionEngine::isProfitableAfterCosts(
    const ArbitrageOpportunity &opp, const std::vector<OrderBook> &books) {

  double total_slippage = 0.0;
  double trade_size = config_.max_trade_usd;

  // books[i] corresponds to opp.market_indices[i], NOT to trade_vector[i].
  for (size_t i = 0; i < books.size() && i < opp.market_indices.size(); i++) {
    size_t mkt_idx = opp.market_indices[i];
    double tv = (mkt_idx < (size_t)opp.trade_vector.size())
                    ? std::abs(opp.trade_vector[mkt_idx])
                    : 0.0;
    if (tv < 1e-6)
      continue;

    Side side = (opp.trade_vector[mkt_idx] > 0) ? Side::BUY : Side::SELL;
    double slippage = estimateSlippage(books[i], side, tv * trade_size);
    total_slippage += slippage * tv;
  }

  // Net profit (USD) calculation
  // opp.expected_profit is a rate (margin/share).
  // Total potential gross profit = margin * volume
  double gross_profit = opp.expected_profit * trade_size;

  // Fees are based on volume traded (not profit)
  double volume = trade_size;
  double fees = volume * config_.fee_rate;

  double net = gross_profit - fees - total_slippage * trade_size;

  spdlog::debug("[Exec] Profitability: gross=${:.2f}, fees=${:.2f}, "
                "slippage=${:.2f}, net=${:.2f}",
                gross_profit, fees, total_slippage * trade_size, net);

  return net >= config_.min_profit_usd;
}

// ── Execute arbitrage ────────────────────────────────────────────────
TradeResult ExecutionEngine::execute(const ArbitrageOpportunity &opp,
                                     const std::vector<Market> &markets) {
  auto start = std::chrono::steady_clock::now();

  TradeResult result;
  result.opportunity_id =
      std::to_string(opp.detected_at.time_since_epoch().count());
  result.expected_pnl = opp.expected_profit;
  result.actual_pnl = 0.0;
  result.total_fees = 0.0;
  result.slippage = 0.0;
  result.fully_filled = true;
  result.status = "PENDING";
  result.executed_at = start;

  double trade_notional = config_.max_trade_usd;

  spdlog::info("[Exec] Executing arbitrage: {} markets, "
               "expected margin={:.4f}",
               opp.market_indices.size(), opp.expected_profit);

  struct PendingLeg {
    std::future<std::optional<std::string>> future;
    Order order;
    double tv;
    double vwap_usd;
  };
  std::vector<PendingLeg> legs;

  for (size_t k = 0; k < opp.market_indices.size(); k++) {
    size_t idx = opp.market_indices[k];
    if (idx >= markets.size())
      continue;

    double tv =
        (idx < (size_t)opp.trade_vector.size()) ? opp.trade_vector[idx] : 0.0;
    if (std::abs(tv) < 1e-6)
      continue;

    const auto &mkt = markets[idx];
    Side side = (tv > 0) ? Side::BUY : Side::SELL;
    std::string token_id =
        (side == Side::BUY) ? mkt.token_id_yes : mkt.token_id_no;

    // Fetch fresh order book for best price
    auto book = feed_.fetchOrderBook(token_id);
    double best_ask = book.bestAsk();
    double best_bid = book.bestBid();

    // Calculate size in SHARES
    // Size (USD) = abs(tv) * trade_notional
    // Size (Shares) = Size (USD) / price
    double usd_size = std::abs(tv) * trade_notional;
    double price = (tv > 0) ? best_ask : best_bid;

    // Safety check for price
    if (price < 0.001 || price > 0.999) {
      spdlog::warn("[Exec] Price {:.3f} extreme, skipping leg {}", price,
                   token_id);
      continue;
    }

    double share_size = usd_size / price;

    if (usd_size < 1.0) // Skip dust < $1
      continue;

    // Estimate VWAP and slippage
    double vwap_usd = computeVWAP(book, side, usd_size);
    double slippage_rate = estimateSlippage(book, side, usd_size);
    result.slippage += slippage_rate;

    // Check latency
    if (elapsed_ms(start) > config_.latency_budget_ms) {
      spdlog::warn("[Exec] Latency budget exceeded ({:.0f}ms), aborting",
                   elapsed_ms(start));
      result.status = "TIMEOUT";
      result.fully_filled = false;
      break;
    }

    Order order;
    order.token_id = token_id;
    order.side = side;
    order.price = price;
    order.size = share_size;
    order.order_id = "PENDING";

    // Submit order asynchronously
    auto fut = std::async(
        std::launch::async, [this, token_id, side, price, share_size]() {
          return feed_.submitOrder(token_id, side, price, share_size);
        });

    legs.push_back({std::move(fut), order, tv, vwap_usd});
  }

  // Wait for results
  for (auto &leg : legs) {
    auto order_id_opt = leg.future.get();
    leg.order.order_id = order_id_opt.value_or("FAILED");
    result.orders.push_back(leg.order);

    if (!order_id_opt) {
      result.fully_filled = false;
      result.status = "PARTIAL";
      spdlog::error("[Exec] Order failed for token: {}", leg.order.token_id);
    } else {
      // Estimate P&L using stored vwap
      // Fee = Volume(USD) * rate
      // Volume(USD) = share_size * vwap_usd (approx execution price)
      // Actually strictly: Volume = size * price.
      // Let's use vwap as execution price proxy.
      double trade_vol = leg.order.size * leg.vwap_usd;
      double fee = trade_vol * config_.fee_rate;
      result.total_fees += fee;

      // P&L = signed_share_quantity * (exit_price - entry_price)
      // Here we are entering. P&L is strictly "value vs cost".
      // But we are arb-ing. The "profit" is the spread captured.
      // The "actual_pnl" field in result is mostly for logging relative to
      // "expected". Expected was "margin * volume". Actual should be "(margin -
      // slippage) * volume - fees". Let's approximate: actual_pnl += (leg.tv *
      // trade_notional) * margin_captured - fee? No. Simply use the
      // "optimization gain" logic: We assume we bought at p* (optimal) but
      // actually bought at vwap. Loss due to slippage = Volume * (vwap - mid).
      // Gain from arb = Volume * margin.
      // Net = Volume * margin - Volume * (vwap - mid) - Fees.
      // This is complicated to reconstruct per leg.
      // Let's just track fees and slippage.
      // The "actual_pnl" logic in original code was: tv * (vwap - price).
      // If tv>0 (Buy), vwap > price (mid). This is positive cost (bad).
      // So (vwap - price) is slippage.
      // tv * slippage = positive * positive = positive slippage cost.
      // We subtract this from expected.
    }
  }

  // Update actual_pnl based on expected - slippage - fees
  // Note: result.slippage is sum(slippage_rate).
  // Total slippage cost ~= sum(slippage_rate * leg_volume).
  // We used "tv" as weight in isProfitable.
  // Let's simplify: actual_pnl = expected_pnl * notional - fees - slippage_cost
  // This is a rough estimate for the log.
  // We can refine this later if needed.
  double total_vol = 0;
  double total_slip_cost = 0;
  for (auto &leg : legs) {
    double vol = leg.order.size * leg.vwap_usd;
    total_vol += vol;
    // Slippage rate * vol
    // We didn't store slippage rate per leg in the struct, but we added to
    // result.slippage. Let's assume average slippage applied to total vol? Or
    // just re-calculate: cost = vol * |vwap - price|/price
    total_slip_cost +=
        vol * std::abs(leg.vwap_usd - leg.order.price) / leg.order.price;
  }

  result.actual_pnl = (result.expected_pnl * trade_notional) -
                      result.total_fees - total_slip_cost;

  if (result.fully_filled) {
    result.status = "FILLED";
  }

  double total_ms = elapsed_ms(start);
  spdlog::info("[Exec] {} in {:.0f}ms: exp=${:.2f}, actual=${:.2f}, "
               "fees=${:.2f}",
               result.status, total_ms, result.expected_pnl * trade_notional,
               result.actual_pnl, result.total_fees);

  return result;
}

} // namespace arbi
