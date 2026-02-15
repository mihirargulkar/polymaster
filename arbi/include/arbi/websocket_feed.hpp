#pragma once

#include "arbi/common.hpp"

#include <atomic>
#include <functional>
#include <mutex>
#include <string>
#include <unordered_map>
#include <vector>

// Forward declaration
namespace ix {
class WebSocket;
struct WebSocketMessage;
} // namespace ix

struct OrderBookUpdate {
  std::string token_id;
  double price;
  double size;
  std::string side; // "BUY" or "SELL"
  long timestamp;
};

// Thread-safe in-memory order book cache fed by WebSocket
class OrderBookCache {
public:
  void update(const std::string &token_id, const arbi::OrderBook &book) {
    std::lock_guard<std::mutex> lock(mu_);
    books_[token_id] = book;
  }

  arbi::OrderBook get(const std::string &token_id) const {
    std::lock_guard<std::mutex> lock(mu_);
    auto it = books_.find(token_id);
    if (it != books_.end())
      return it->second;
    arbi::OrderBook empty;
    empty.token_id = token_id;
    return empty;
  }

  bool has(const std::string &token_id) const {
    std::lock_guard<std::mutex> lock(mu_);
    return books_.count(token_id) > 0;
  }

  size_t size() const {
    std::lock_guard<std::mutex> lock(mu_);
    return books_.size();
  }

private:
  mutable std::mutex mu_;
  std::unordered_map<std::string, arbi::OrderBook> books_;
};

class WebSocketFeed {
public:
  WebSocketFeed();
  ~WebSocketFeed();

  // Connect to Polymarket CLOB WebSocket
  void setup();
  void connect();

  // Subscribe to market updates for a list of token IDs
  void subscribe(const std::vector<std::string> &token_ids);

  // Set callback for price updates (simple price events)
  void setOnUpdate(std::function<void(const OrderBookUpdate &)> callback);

  // Access the order book cache (thread-safe)
  OrderBookCache &orderBookCache() { return ob_cache_; }

private:
  std::string url_;
  std::unique_ptr<ix::WebSocket> ws_;
  std::atomic<bool> connected_{false};
  std::function<void(const OrderBookUpdate &)> update_callback_;
  OrderBookCache ob_cache_;

  void onMessage(const ix::WebSocketMessage &msg);
};
