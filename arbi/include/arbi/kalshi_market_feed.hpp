#pragma once

#include "arbi/common.hpp"
#include <string>
#include <vector>

namespace arbi {

class KalshiMarketFeed {
public:
  KalshiMarketFeed(const std::string &key_id,
                   const std::string &private_key_path);
  ~KalshiMarketFeed();

  // Fetch open Kalshi markets via REST API
  std::vector<Market> fetchMarkets(int limit = 200);

  // Match Kalshi markets against Polymarket markets by title similarity
  static std::vector<CrossExchangePair>
  matchMarkets(const std::vector<Market> &poly_markets,
               const std::vector<Market> &kalshi_markets,
               double min_similarity = 0.4);

  // REST: fetch order book for a ticker
  OrderBook fetchOrderBook(const std::string &ticker);

  // REST: submit an order (live or paper)
  std::optional<std::string> submitOrder(const std::string &ticker, Side side,
                                         double price, double count);

private:
  std::string httpGet(const std::string &url);
  std::string httpPost(const std::string &url, const std::string &body);
  std::string signRequest(const std::string &timestamp,
                          const std::string &method, const std::string &path);
  std::string base64Encode(const unsigned char *buffer, size_t length);

  std::string key_id_;
  std::string private_key_path_;
  void *pkey_ = nullptr; // EVP_PKEY*

  static std::vector<std::string> tokenize(const std::string &text);
  static double jaccardSimilarity(const std::vector<std::string> &a,
                                  const std::vector<std::string> &b);
};

} // namespace arbi
