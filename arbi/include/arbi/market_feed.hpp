#pragma once
#include "arbi/common.hpp"
#include <functional>

namespace arbi {

class MarketFeed {
public:
  explicit MarketFeed(const Config &config);
  ~MarketFeed();

  // REST: fetch all active markets
  std::vector<Market> fetchMarkets();

  // REST: fetch order book for a token
  OrderBook fetchOrderBook(const std::string &token_id);

  // REST: fetch order books for multiple tokens
  std::unordered_map<std::string, OrderBook>
  fetchOrderBooks(const std::vector<std::string> &token_ids);

  // Submit order (live or paper)
  std::optional<std::string> submitOrder(const std::string &token_id, Side side,
                                         double price, double size);

private:
  Config config_;
  std::string baseUrl_ = "https://clob.polymarket.com";

  // HTTP helpers
  std::string httpGet(const std::string &url);
  std::string httpPost(const std::string &url, const std::string &body);
};

} // namespace arbi
