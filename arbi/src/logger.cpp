#include "arbi/logger.hpp"
#include <ctime>
#include <filesystem>
#include <iomanip>
#include <spdlog/spdlog.h>
#include <sstream>

namespace arbi {

static std::string timestamp() {
  auto now = std::chrono::system_clock::now();
  auto t = std::chrono::system_clock::to_time_t(now);
  auto ms = std::chrono::duration_cast<std::chrono::milliseconds>(
                now.time_since_epoch()) %
            1000;
  std::ostringstream ss;
  ss << std::put_time(std::localtime(&t), "%Y-%m-%dT%H:%M:%S") << '.'
     << std::setfill('0') << std::setw(3) << ms.count();
  return ss.str();
}

Logger::Logger(const std::string &log_dir) : log_dir_(log_dir) {
  std::filesystem::create_directories(log_dir_);

  trade_csv_.open(log_dir_ + "/trades.csv", std::ios::app);
  opp_csv_.open(log_dir_ + "/opportunities.csv", std::ios::app);
  ensureHeaders();
}

Logger::~Logger() {
  if (trade_csv_.is_open())
    trade_csv_.close();
  if (opp_csv_.is_open())
    opp_csv_.close();
}

void Logger::ensureHeaders() {
  // Use file_size to check if files are empty (tellp() is unreliable with
  // ios::app)
  auto trade_path = std::filesystem::path(log_dir_) / "trades.csv";
  auto opp_path = std::filesystem::path(log_dir_) / "opportunities.csv";

  if (std::filesystem::file_size(trade_path) == 0) {
    trade_csv_ << "timestamp,opportunity_id,status,expected_pnl,"
                  "actual_pnl,fees,slippage,num_orders\n";
  }
  if (std::filesystem::file_size(opp_path) == 0) {
    opp_csv_ << "timestamp,num_markets,expected_profit,"
                "mispricing_pct,trade_vector_norm\n";
  }
}

void Logger::logOpportunity(const ArbitrageOpportunity &opp,
                            const std::vector<Market> &markets) {
  std::lock_guard<std::mutex> lock(mtx_);
  auto ts = timestamp();
  double norm = opp.trade_vector.norm();

  opp_csv_ << ts << "," << opp.market_indices.size() << "," << std::fixed
           << std::setprecision(6) << opp.expected_profit << "," << std::fixed
           << std::setprecision(4) << opp.mispricing_pct << "," << std::fixed
           << std::setprecision(6) << norm << "\n";
  opp_csv_.flush();

  // Also log market details to spdlog
  spdlog::info("💰 Arbitrage found: profit=${:.4f}, mispricing={:.1f}%, "
               "markets={}",
               opp.expected_profit, opp.mispricing_pct * 100,
               opp.market_indices.size());

  for (size_t idx : opp.market_indices) {
    if (idx < markets.size()) {
      spdlog::info("  ├─ {}: YES={:.3f} NO={:.3f}",
                   markets[idx].question.substr(0, 60), markets[idx].yes_price,
                   markets[idx].no_price);
    }
  }
}

void Logger::logTrade(const TradeResult &result) {
  std::lock_guard<std::mutex> lock(mtx_);
  auto ts = timestamp();

  trade_csv_ << ts << "," << result.opportunity_id << "," << result.status
             << "," << std::fixed << std::setprecision(6) << result.expected_pnl
             << "," << std::fixed << std::setprecision(6) << result.actual_pnl
             << "," << std::fixed << std::setprecision(6) << result.total_fees
             << "," << std::fixed << std::setprecision(6) << result.slippage
             << "," << result.orders.size() << "\n";
  trade_csv_.flush();

  if (result.status == "FILLED") {
    spdlog::info("✅ Trade executed: expected=${:.4f}, actual=${:.4f}, "
                 "fees=${:.4f}",
                 result.expected_pnl, result.actual_pnl, result.total_fees);
  } else {
    spdlog::warn("⚠️  Trade {}: {}", result.status, result.opportunity_id);
  }
}

void Logger::logCycle(int cycle, int markets_scanned, int opportunities_found,
                      double elapsed) {
  spdlog::info("── Cycle {} ── markets={}, opportunities={}, "
               "elapsed={:.1f}ms ──",
               cycle, markets_scanned, opportunities_found, elapsed);
}

} // namespace arbi
