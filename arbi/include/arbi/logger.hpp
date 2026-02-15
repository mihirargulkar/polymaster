#pragma once
#include "arbi/common.hpp"
#include <fstream>
#include <mutex>
#include <string>

namespace arbi {

class Logger {
public:
  explicit Logger(const std::string &log_dir = "logs");
  ~Logger();

  void logOpportunity(const ArbitrageOpportunity &opp,
                      const std::vector<Market> &markets);
  void logTrade(const TradeResult &result);
  void logCycle(int cycle, int markets_scanned, int opportunities_found,
                double elapsed_ms);

private:
  std::string log_dir_;
  std::ofstream trade_csv_;
  std::ofstream opp_csv_;
  std::mutex mtx_;

  void ensureHeaders();
};

} // namespace arbi
