// ═══════════════════════════════════════════════════════════════════════
//  ARBI — Comprehensive Unit Test Suite
//  Tests each component in isolation with synthetic data — no network calls
// ═══════════════════════════════════════════════════════════════════════
#include "arbi/bregman.hpp"
#include "arbi/common.hpp"
#include "arbi/frank_wolfe.hpp"
#include "arbi/logger.hpp"
#include "arbi/polytope.hpp"

#include <Eigen/Dense>
#include <spdlog/spdlog.h>

#include <cassert>
#include <cmath>
#include <filesystem>
#include <fstream>
#include <functional>
#include <iostream>
#include <string>
#include <vector>

using namespace arbi;

static int g_passed = 0;
static int g_failed = 0;

static void runTest(const char *name, std::function<void()> fn) {
  std::cout << "  ├─ " << name << "... " << std::flush;
  try {
    fn();
    std::cout << "✅\n";
    g_passed++;
  } catch (const std::exception &e) {
    std::cout << "❌ " << e.what() << "\n";
    g_failed++;
  }
}

#define ASSERT_TRUE(cond)                                                      \
  do {                                                                         \
    if (!(cond))                                                               \
      throw std::runtime_error(std::string("ASSERT_TRUE failed: ") + #cond +   \
                               " at line " + std::to_string(__LINE__));        \
  } while (0)

#define ASSERT_FALSE(cond) ASSERT_TRUE(!(cond))

#define ASSERT_NEAR(a, b, eps)                                                 \
  do {                                                                         \
    if (std::abs((a) - (b)) > (eps))                                           \
      throw std::runtime_error("ASSERT_NEAR failed: " + std::to_string(a) +    \
                               " != " + std::to_string(b) +                    \
                               " (eps=" + std::to_string(eps) + ") at line " + \
                               std::to_string(__LINE__));                      \
  } while (0)

#define ASSERT_EQ(a, b) ASSERT_TRUE((a) == (b))

// ═══════════════════════════════════════════════════════════════════════
//  Inline VWAP/slippage (mirrors ExecutionEngine logic, no network)
// ═══════════════════════════════════════════════════════════════════════
static double testVWAP(const OrderBook &book, Side side, double size) {
  const auto &levels = (side == Side::BUY) ? book.asks : book.bids;
  if (levels.empty())
    return 0.0;
  double remaining = size, total_cost = 0.0, total_filled = 0.0;
  for (auto &level : levels) {
    double fill = std::min(remaining, level.size);
    total_cost += fill * level.price;
    total_filled += fill;
    remaining -= fill;
    if (remaining <= 0)
      break;
  }
  return (total_filled == 0.0) ? 0.0 : total_cost / total_filled;
}

static double testSlippage(const OrderBook &book, Side side, double size) {
  double vwap = testVWAP(book, side, size);
  double best = (side == Side::BUY) ? book.bestAsk() : book.bestBid();
  if (best == 0.0)
    return 1.0;
  return std::abs(vwap - best) / best;
}

// ═══════════════════════════════════════════════════════════════════════
int main() {
  spdlog::set_level(spdlog::level::off);

  std::cout << "\n╔══════════════════════════════════════════════╗\n";
  std::cout << "║        ARBI — Unit Test Suite                ║\n";
  std::cout << "╚══════════════════════════════════════════════╝\n";

  // ─── 1. COMMON / DATA STRUCTURES ────────────────────────────────
  std::cout << "\n📦 common.hpp\n";

  runTest("orderbook_best_bid_ask", [] {
    OrderBook book;
    book.bids = {{0.55, 100}, {0.50, 200}};
    book.asks = {{0.60, 100}, {0.65, 200}};
    ASSERT_NEAR(book.bestBid(), 0.55, 1e-9);
    ASSERT_NEAR(book.bestAsk(), 0.60, 1e-9);
    ASSERT_NEAR(book.midpoint(), 0.575, 1e-9);
    ASSERT_NEAR(book.spread(), 0.05, 1e-9);
  });

  runTest("orderbook_empty", [] {
    OrderBook book;
    ASSERT_NEAR(book.bestBid(), 0.0, 1e-9);
    ASSERT_NEAR(book.bestAsk(), 1.0, 1e-9);
    ASSERT_NEAR(book.midpoint(), 0.5, 1e-9);
    ASSERT_NEAR(book.spread(), 1.0, 1e-9);
  });

  runTest("elapsed_ms_positive", [] {
    auto start = std::chrono::steady_clock::now();
    volatile int x = 0;
    for (int i = 0; i < 100000; i++)
      x += i;
    (void)x;
    double ms = elapsed_ms(start);
    ASSERT_TRUE(ms >= 0.0);
  });

  runTest("config_defaults", [] {
    Config cfg;
    ASSERT_FALSE(cfg.live_mode);
    ASSERT_NEAR(cfg.max_trade_usd, 100.0, 1e-9);
    ASSERT_NEAR(cfg.fee_rate, 0.02, 1e-9);
    ASSERT_EQ(cfg.scan_interval_s, 1);
    ASSERT_EQ(cfg.fw_max_iters, 150);
    ASSERT_NEAR(cfg.fw_tolerance, 1e-8, 1e-15);
    ASSERT_NEAR(cfg.min_profit_usd, 0.50, 1e-9);
    ASSERT_TRUE(cfg.groq_api_key.empty());
  });

  // ─── 2. MARGINAL POLYTOPE ───────────────────────────────────────
  std::cout << "\n📐 polytope.cpp\n";

  runTest("polytope_no_constraints", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps;
    poly.buildConstraints(3, deps);
    ASSERT_EQ(poly.numConstraints(), (size_t)0);
    ASSERT_EQ(poly.numVariables(), (size_t)3);
    Eigen::VectorXd p(3);
    p << 0.3, 0.7, 0.5;
    auto res = poly.checkFeasibility(p);
    ASSERT_TRUE(res.feasible);
    ASSERT_NEAR(res.violation, 0.0, 1e-9);
  });

  runTest("polytope_mutex_feasible", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps = {{0, 1, Relation::MUTEX}};
    poly.buildConstraints(2, deps);
    ASSERT_EQ(poly.numConstraints(), (size_t)1);
    Eigen::VectorXd p(2);
    p << 0.3, 0.5;
    auto res = poly.checkFeasibility(p);
    ASSERT_TRUE(res.feasible);
  });

  runTest("polytope_mutex_infeasible", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps = {{0, 1, Relation::MUTEX}};
    poly.buildConstraints(2, deps);
    Eigen::VectorXd p(2);
    p << 0.6, 0.7;
    auto res = poly.checkFeasibility(p);
    ASSERT_FALSE(res.feasible);
    ASSERT_TRUE(res.violation > 0.0);
    ASSERT_NEAR(res.violation, 0.3, 0.1);
  });

  runTest("polytope_implies_feasible", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps = {{0, 1, Relation::IMPLIES}};
    poly.buildConstraints(2, deps);
    // A(0) implies B(1) => P(0) <= P(1)
    Eigen::VectorXd p(2);
    p << 0.3, 0.7; // 0.3 <= 0.7 -> Feasible
    auto res = poly.checkFeasibility(p);
    ASSERT_TRUE(res.feasible);
  });

  runTest("polytope_implies_infeasible", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps = {{0, 1, Relation::IMPLIES}};
    poly.buildConstraints(2, deps);
    // A(0) implies B(1) => P(0) <= P(1)
    Eigen::VectorXd p(2);
    p << 0.8, 0.3; // 0.8 > 0.3 -> Infeasible
    auto res = poly.checkFeasibility(p);
    ASSERT_FALSE(res.feasible);
    ASSERT_NEAR(res.violation, 0.5, 0.1);
  });

  runTest("polytope_exactly_one_feasible", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps = {{0, 1, Relation::EXACTLY_ONE}};
    poly.buildConstraints(2, deps);
    Eigen::VectorXd p(2);
    p << 0.4, 0.6;
    auto res = poly.checkFeasibility(p);
    ASSERT_TRUE(res.feasible);
  });

  runTest("polytope_exactly_one_infeasible", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps = {{0, 1, Relation::EXACTLY_ONE}};
    poly.buildConstraints(2, deps);
    Eigen::VectorXd p(2);
    p << 0.3, 0.4;
    auto res = poly.checkFeasibility(p);
    ASSERT_FALSE(res.feasible);
  });

  runTest("polytope_lp_solve_basic", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps = {{0, 1, Relation::MUTEX}};
    poly.buildConstraints(2, deps);
    Eigen::VectorXd obj(2);
    obj << 1.0, 1.0;
    auto sol = poly.solveLP(obj);
    ASSERT_TRUE(sol.has_value());
    ASSERT_NEAR((*sol)[0], 0.0, 1e-6);
    ASSERT_NEAR((*sol)[1], 0.0, 1e-6);
  });

  runTest("polytope_lp_maximize_direction", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps = {{0, 1, Relation::MUTEX}};
    poly.buildConstraints(2, deps);
    Eigen::VectorXd obj(2);
    obj << -1.0, 1.0;
    auto sol = poly.solveLP(obj);
    ASSERT_TRUE(sol.has_value());
    ASSERT_NEAR((*sol)[0], 1.0, 1e-6);
    ASSERT_NEAR((*sol)[1], 0.0, 1e-6);
  });

  runTest("polytope_multiple_constraints", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps = {{0, 1, Relation::MUTEX},
                                    {2, 1, Relation::IMPLIES}};
    poly.buildConstraints(3, deps);
    ASSERT_EQ(poly.numConstraints(), (size_t)2);
    Eigen::VectorXd p(3);
    // 0+1 <= 1 (0.3+0.4=0.7 OK)
    // 2 <= 1 (0.3 <= 0.4 OK)
    p << 0.3, 0.4, 0.3;
    auto res = poly.checkFeasibility(p);
    ASSERT_TRUE(res.feasible);
  });

  runTest("polytope_independent_skip", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps = {{0, 1, Relation::INDEPENDENT}};
    poly.buildConstraints(2, deps);
    ASSERT_EQ(poly.numConstraints(), (size_t)0);
  });

  // ─── 3. BREGMAN PROJECTION ──────────────────────────────────────
  std::cout << "\n🔬 bregman.cpp\n";

  runTest("kl_divergence_identical", [] {
    Eigen::VectorXd p(3);
    p << 0.5, 0.3, 0.7;
    double kl = BregmanProjection::klDivergence(p, p);
    ASSERT_NEAR(kl, 0.0, 1e-9);
  });

  runTest("kl_divergence_positive", [] {
    Eigen::VectorXd q(2), p(2);
    q << 0.6, 0.4;
    p << 0.5, 0.5;
    double kl = BregmanProjection::klDivergence(q, p);
    ASSERT_TRUE(kl > 0.0);
  });

  runTest("kl_divergence_asymmetric", [] {
    Eigen::VectorXd q(2), p(2);
    q << 0.8, 0.2;
    p << 0.5, 0.5;
    double kl1 = BregmanProjection::klDivergence(q, p);
    double kl2 = BregmanProjection::klDivergence(p, q);
    ASSERT_TRUE(std::abs(kl1 - kl2) > 1e-6);
  });

  runTest("kl_gradient_at_same_point", [] {
    Eigen::VectorXd p(3);
    p << 0.5, 0.3, 0.7;
    auto grad = BregmanProjection::klGradient(p, p);
    for (int i = 0; i < grad.size(); i++) {
      ASSERT_NEAR(grad[i], 0.0, 1e-6);
    }
  });

  runTest("kl_divergence_extreme_values", [] {
    Eigen::VectorXd q(2), p(2);
    q << 0.001, 0.999;
    p << 0.999, 0.001;
    double kl = BregmanProjection::klDivergence(q, p);
    ASSERT_TRUE(std::isfinite(kl));
    ASSERT_TRUE(kl > 0.0);
  });

  runTest("bregman_project_feasible", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps = {{0, 1, Relation::MUTEX}};
    poly.buildConstraints(2, deps);
    Eigen::VectorXd p(2);
    p << 0.3, 0.4;
    BregmanProjection bp;
    auto res = bp.project(p, poly, 100, 1e-8);
    ASSERT_NEAR(res.projected[0], 0.3, 0.05);
    ASSERT_NEAR(res.projected[1], 0.4, 0.05);
    ASSERT_TRUE(res.divergence < 0.01);
  });

  runTest("bregman_project_infeasible", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps = {{0, 1, Relation::MUTEX}};
    poly.buildConstraints(2, deps);
    Eigen::VectorXd p(2);
    p << 0.7, 0.6;
    BregmanProjection bp;
    auto res = bp.project(p, poly, 200, 1e-10);
    ASSERT_TRUE(res.projected[0] + res.projected[1] <= 1.0 + 1e-3);
    ASSERT_TRUE(res.divergence > 0.0);
  });

  // ─── 4. FRANK-WOLFE OPTIMIZER ───────────────────────────────────
  std::cout << "\n⚡ frank_wolfe.cpp\n";

  runTest("fw_feasible_no_profit", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps = {{0, 1, Relation::MUTEX}};
    poly.buildConstraints(2, deps);
    Eigen::VectorXd p(2);
    p << 0.3, 0.4;
    FrankWolfe fw;
    auto res = fw.optimize(p, poly, 100, 1e-8);
    ASSERT_TRUE(res.profit < 0.05);
    ASSERT_TRUE(res.elapsed_ms >= 0.0);
  });

  runTest("fw_infeasible_finds_profit", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps = {{0, 1, Relation::MUTEX}};
    poly.buildConstraints(2, deps);
    Eigen::VectorXd p(2);
    p << 0.7, 0.6;
    FrankWolfe fw;
    auto res = fw.optimize(p, poly, 200, 1e-10);
    ASSERT_TRUE(res.profit > 0.0);
    ASSERT_TRUE(res.iterations > 0);
    ASSERT_TRUE(res.optimal[0] + res.optimal[1] <= 1.0 + 1e-3);
  });

  runTest("fw_trade_vector_correct", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps = {{0, 1, Relation::MUTEX}};
    poly.buildConstraints(2, deps);
    Eigen::VectorXd p(2);
    p << 0.7, 0.6;
    FrankWolfe fw;
    auto res = fw.optimize(p, poly, 200, 1e-10);
    for (int i = 0; i < 2; i++) {
      ASSERT_NEAR(res.trade_vector[i], res.optimal[i] - p[i], 1e-9);
    }
  });

  runTest("fw_convergence", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps = {{0, 1, Relation::MUTEX}};
    poly.buildConstraints(2, deps);
    Eigen::VectorXd p(2);
    p << 0.7, 0.6;
    FrankWolfe fw;
    auto res = fw.optimize(p, poly, 1000, 1e-6);
    // Should converge eventually (exact line search from center is slower)
    ASSERT_TRUE(res.converged);
  });

  runTest("fw_larger_system", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps = {{0, 1, Relation::MUTEX},
                                    {2, 3, Relation::MUTEX},
                                    {2, 0, Relation::IMPLIES}};
    poly.buildConstraints(4, deps);
    ASSERT_EQ(poly.numConstraints(), (size_t)3);
    Eigen::VectorXd p(4);
    p << 0.8, 0.5, 0.9, 0.3;
    FrankWolfe fw;
    auto res = fw.optimize(p, poly, 300, 1e-10);
    ASSERT_TRUE(res.profit > 0.0);
    ASSERT_TRUE(res.optimal[0] + res.optimal[1] <= 1.0 + 1e-3);
    ASSERT_TRUE(res.optimal[2] + res.optimal[3] <= 1.0 + 1e-3);
  });

  // ─── 5. VWAP / SLIPPAGE ─────────────────────────────────────────
  std::cout << "\n💹 VWAP / slippage\n";

  runTest("vwap_single_level", [] {
    OrderBook book;
    book.asks = {{0.60, 100}};
    double vwap = testVWAP(book, Side::BUY, 50);
    ASSERT_NEAR(vwap, 0.60, 1e-9);
  });

  runTest("vwap_multi_level", [] {
    OrderBook book;
    book.asks = {{0.60, 50}, {0.65, 50}, {0.70, 100}};
    double vwap = testVWAP(book, Side::BUY, 80);
    ASSERT_NEAR(vwap, (50 * 0.60 + 30 * 0.65) / 80.0, 1e-9);
  });

  runTest("vwap_exceeds_book", [] {
    OrderBook book;
    book.asks = {{0.60, 10}};
    double vwap = testVWAP(book, Side::BUY, 20);
    ASSERT_NEAR(vwap, 0.60, 1e-9);
  });

  runTest("vwap_sell_bids", [] {
    OrderBook book;
    book.bids = {{0.55, 100}, {0.50, 200}};
    double vwap = testVWAP(book, Side::SELL, 150);
    ASSERT_NEAR(vwap, (100 * 0.55 + 50 * 0.50) / 150.0, 1e-9);
  });

  runTest("slippage_no_impact", [] {
    OrderBook book;
    book.asks = {{0.60, 1000}};
    double slip = testSlippage(book, Side::BUY, 10);
    ASSERT_NEAR(slip, 0.0, 1e-9);
  });

  runTest("slippage_with_impact", [] {
    OrderBook book;
    book.asks = {{0.60, 10}, {0.65, 100}};
    double slip = testSlippage(book, Side::BUY, 20);
    ASSERT_TRUE(slip > 0.0);
    double expected_vwap = (10 * 0.60 + 10 * 0.65) / 20.0;
    ASSERT_NEAR(slip, (expected_vwap - 0.60) / 0.60, 1e-9);
  });

  runTest("slippage_empty_book", [] {
    OrderBook book;
    double slip = testSlippage(book, Side::SELL, 10);
    ASSERT_NEAR(slip, 1.0, 1e-9);
  });

  // ─── 6. LOGGER ──────────────────────────────────────────────────
  std::cout << "\n📝 logger.cpp\n";

  runTest("logger_creates_files", [] {
    std::string td = "/tmp/arbi_test_logs_1";
    std::filesystem::remove_all(td);
    {
      Logger logger(td);
    }
    ASSERT_TRUE(std::filesystem::exists(td + "/trades.csv"));
    ASSERT_TRUE(std::filesystem::exists(td + "/opportunities.csv"));
    std::filesystem::remove_all(td);
  });

  runTest("logger_writes_trade", [] {
    std::string td = "/tmp/arbi_test_logs_2";
    std::filesystem::remove_all(td);
    {
      Logger logger(td);
      TradeResult tr;
      tr.opportunity_id = "TEST_OPP_42";
      tr.status = "FILLED";
      tr.expected_pnl = 1.5;
      tr.actual_pnl = 1.2;
      tr.total_fees = 0.1;
      tr.slippage = 0.05;
      tr.executed_at = std::chrono::steady_clock::now();
      logger.logTrade(tr);
    }
    std::ifstream f(td + "/trades.csv");
    ASSERT_TRUE(f.is_open());
    std::string content((std::istreambuf_iterator<char>(f)),
                        std::istreambuf_iterator<char>());
    ASSERT_TRUE(content.find("TEST_OPP_42") != std::string::npos);
    ASSERT_TRUE(content.find("FILLED") != std::string::npos);
    std::filesystem::remove_all(td);
  });

  runTest("logger_writes_opportunity", [] {
    std::string td = "/tmp/arbi_test_logs_3";
    std::filesystem::remove_all(td);
    {
      Logger logger(td);
      ArbitrageOpportunity opp;
      opp.market_indices = {0, 1};
      opp.trade_vector = Eigen::VectorXd(2);
      opp.trade_vector << -0.05, -0.05;
      opp.expected_profit = 0.123;
      opp.mispricing_pct = 0.07;
      opp.detected_at = std::chrono::steady_clock::now();
      std::vector<Market> markets(2);
      markets[0].question = "Test Market A";
      markets[0].yes_price = 0.5;
      markets[0].no_price = 0.5;
      markets[1].question = "Test Market B";
      markets[1].yes_price = 0.6;
      markets[1].no_price = 0.4;
      logger.logOpportunity(opp, markets);
    }
    std::ifstream f(td + "/opportunities.csv");
    std::string content((std::istreambuf_iterator<char>(f)),
                        std::istreambuf_iterator<char>());
    ASSERT_TRUE(content.find("0.1230") != std::string::npos);
    std::filesystem::remove_all(td);
  });

  // ─── 7. END-TO-END PIPELINE ─────────────────────────────────────
  std::cout << "\n🔗 End-to-end pipeline\n";

  runTest("e2e_two_market_arb", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps = {{0, 1, Relation::MUTEX}};
    poly.buildConstraints(2, deps);
    Eigen::VectorXd prices(2);
    prices << 0.7, 0.6;
    auto feas = poly.checkFeasibility(prices);
    ASSERT_FALSE(feas.feasible);
    FrankWolfe fw;
    auto res = fw.optimize(prices, poly, 300, 1e-10);
    ASSERT_TRUE(res.profit > 0.0);
    ASSERT_TRUE(res.optimal[0] + res.optimal[1] <= 1.0 + 1e-3);
    ASSERT_TRUE(res.trade_vector.norm() > 1e-6);
  });

  runTest("e2e_three_market_chain", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps = {{1, 0, Relation::IMPLIES},
                                    {2, 1, Relation::IMPLIES}};
    poly.buildConstraints(3, deps);
    poly.buildConstraints(3, deps);
    Eigen::VectorXd prices(3);
    // Chain: 2 <= 1 <= 0
    // Violate it: P(2)=0.8, P(0)=0.3
    // 0.8 > 0.5 > 0.3 (Reverse order -> Arbitrage)
    prices << 0.3, 0.5, 0.8;
    auto feas = poly.checkFeasibility(prices);
    ASSERT_FALSE(feas.feasible);
    FrankWolfe fw;
    auto res = fw.optimize(prices, poly, 300, 1e-10);
    ASSERT_TRUE(res.profit > 0.0);
    ASSERT_TRUE(res.optimal[0] <= res.optimal[1] + 1e-3);
    ASSERT_TRUE(res.optimal[1] <= res.optimal[2] + 1e-3);
  });

  runTest("e2e_fair_no_arb", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps = {{0, 1, Relation::MUTEX}};
    poly.buildConstraints(2, deps);
    Eigen::VectorXd prices(2);
    prices << 0.3, 0.4;
    auto feas = poly.checkFeasibility(prices);
    ASSERT_TRUE(feas.feasible);
    FrankWolfe fw;
    auto res = fw.optimize(prices, poly, 100, 1e-8);
    ASSERT_TRUE(res.profit < 0.01);
  });

  // ─── 8. EDGE CASES ──────────────────────────────────────────────
  std::cout << "\n🧪 Edge cases\n";

  runTest("polytope_boundary_prices", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps = {{0, 1, Relation::MUTEX}};
    poly.buildConstraints(2, deps);
    Eigen::VectorXd p(2);
    p << 0.5, 0.5;
    auto res = poly.checkFeasibility(p);
    ASSERT_TRUE(res.feasible);
  });

  runTest("polytope_zero_prices", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps = {{0, 1, Relation::MUTEX}};
    poly.buildConstraints(2, deps);
    Eigen::VectorXd p(2);
    p << 0.0, 0.0;
    auto res = poly.checkFeasibility(p);
    ASSERT_TRUE(res.feasible);
  });

  runTest("polytope_one_prices", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps = {{0, 1, Relation::MUTEX}};
    poly.buildConstraints(2, deps);
    Eigen::VectorXd p(2);
    p << 1.0, 1.0;
    auto res = poly.checkFeasibility(p);
    ASSERT_FALSE(res.feasible);
  });

  runTest("fw_single_market_no_constraints", [] {
    MarginalPolytope poly;
    std::vector<Dependency> deps;
    poly.buildConstraints(1, deps);
    Eigen::VectorXd p(1);
    p << 0.5;
    FrankWolfe fw;
    auto res = fw.optimize(p, poly, 50, 1e-8);
    ASSERT_TRUE(std::isfinite(res.profit));
  });

  runTest("kl_binary_large_distance", [] {
    Eigen::VectorXd q(1), p(1);
    q << 0.8;
    p << 0.2;
    double kl = BregmanProjection::klDivergence(q, p);
    ASSERT_TRUE(kl > 0.0);
    ASSERT_TRUE(std::isfinite(kl));
  });

  // ─── RESULTS ────────────────────────────────────────────────────
  std::cout << "\n══════════════════════════════════════════════\n";
  std::cout << "Results: " << g_passed << " passed, " << g_failed << " failed, "
            << (g_passed + g_failed) << " total\n";
  std::cout << "══════════════════════════════════════════════\n\n";

  return g_failed > 0 ? 1 : 0;
}
