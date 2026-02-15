#include "arbi/websocket_feed.hpp"
#include <algorithm>
#include <ixwebsocket/IXUserAgent.h>
#include <ixwebsocket/IXWebSocket.h>
#include <memory>
#include <nlohmann/json.hpp>
#include <spdlog/spdlog.h>

using json = nlohmann::json;
using namespace arbi;

WebSocketFeed::WebSocketFeed() {
  ws_ = std::make_unique<ix::WebSocket>();
  setup();
}

WebSocketFeed::~WebSocketFeed() {
  if (ws_) {
    ws_->stop();
  }
}

void WebSocketFeed::setup() {
  url_ = "wss://ws-subscriptions-clob.polymarket.com/ws/market";

  ws_->setUrl(url_);

  // Optional: heartbeat
  ws_->setPingInterval(30);

  ws_->setOnMessageCallback(
      [this](const ix::WebSocketMessagePtr &msg) { onMessage(*msg); });
}

void WebSocketFeed::connect() {
  spdlog::info("Connecting to WebSocket at {}...", url_);
  ws_->start();
}

void WebSocketFeed::subscribe(const std::vector<std::string> &token_ids) {
  if (!ws_) {
    spdlog::error("WS not initialized");
    return;
  }

  // Polymarket CLOB Market Channel Subscription
  json sub_msg;
  sub_msg["assets_ids"] = token_ids;
  sub_msg["type"] = "market";

  std::string payload = sub_msg.dump();
  spdlog::info("Subscribing to {} assets...", token_ids.size());
  ws_->send(payload);
}

void WebSocketFeed::setOnUpdate(
    std::function<void(const OrderBookUpdate &)> callback) {
  update_callback_ = callback;
}

// Helper: parse a buys/sells array into OrderBookLevel vector
static std::vector<OrderBookLevel> parseLevels(const json &arr) {
  std::vector<OrderBookLevel> levels;
  for (const auto &entry : arr) {
    double price = 0.0, size = 0.0;
    if (entry.contains("price")) {
      if (entry["price"].is_string())
        price = std::stod(entry["price"].get<std::string>());
      else
        price = entry["price"].get<double>();
    }
    if (entry.contains("size")) {
      if (entry["size"].is_string())
        size = std::stod(entry["size"].get<std::string>());
      else
        size = entry["size"].get<double>();
    }
    levels.push_back({price, size});
  }
  return levels;
}

void WebSocketFeed::onMessage(const ix::WebSocketMessage &msg) {
  if (msg.type == ix::WebSocketMessageType::Open) {
    spdlog::info("WebSocket Connected!");
    connected_ = true;
  } else if (msg.type == ix::WebSocketMessageType::Message) {
    try {
      auto j = json::parse(msg.str);

      // ── Handle "book" events (Level 2 order book snapshot) ──
      if (j.is_object() && j.contains("event_type") &&
          j["event_type"] == "book") {
        std::string asset_id = j.value("asset_id", "");
        if (asset_id.empty())
          return;

        OrderBook book;
        book.token_id = asset_id;

        if (j.contains("buys") && j["buys"].is_array()) {
          book.bids = parseLevels(j["buys"]);
          // Sort bids descending by price
          std::sort(book.bids.begin(), book.bids.end(),
                    [](auto &a, auto &b) { return a.price > b.price; });
        }
        if (j.contains("sells") && j["sells"].is_array()) {
          book.asks = parseLevels(j["sells"]);
          // Sort asks ascending by price
          std::sort(book.asks.begin(), book.asks.end(),
                    [](auto &a, auto &b) { return a.price < b.price; });
        }

        ob_cache_.update(asset_id, book);

        // Also fire callback with midpoint price for the price cache
        if (update_callback_) {
          OrderBookUpdate update;
          update.token_id = asset_id;
          update.price = book.midpoint();
          update_callback_(update);
        }
        return;
      }

      // ── Handle "price_change" or "last_trade_price" events ──
      if (j.is_object() && j.contains("event_type") &&
          j["event_type"] == "last_trade_price") {
        std::string asset_id = j.value("asset_id", "");
        if (asset_id.empty())
          return;

        double price = 0.0;
        if (j.contains("price")) {
          if (j["price"].is_string())
            price = std::stod(j["price"].get<std::string>());
          else
            price = j["price"].get<double>();
        }

        if (update_callback_ && !asset_id.empty()) {
          OrderBookUpdate update;
          update.token_id = asset_id;
          update.price = price;
          update_callback_(update);
        }
        return;
      }

      // ── Handle "tick_size_change" or "price_change" events ──
      if (j.is_object() && j.contains("event_type") &&
          j["event_type"] == "price_change") {
        std::string asset_id = j.value("asset_id", "");
        double price = 0.0;
        if (j.contains("price")) {
          if (j["price"].is_string())
            price = std::stod(j["price"].get<std::string>());
          else
            price = j["price"].get<double>();
        }
        if (update_callback_ && !asset_id.empty()) {
          OrderBookUpdate update;
          update.token_id = asset_id;
          update.price = price;
          update_callback_(update);
        }
        return;
      }

      // ── Handle arrays of updates ──
      if (j.is_array()) {
        for (const auto &item : j) {
          OrderBookUpdate update;
          if (item.contains("asset_id")) {
            update.token_id = item["asset_id"].get<std::string>();
          } else if (item.contains("token_id")) {
            update.token_id = item["token_id"].get<std::string>();
          } else {
            continue;
          }
          if (item.contains("price")) {
            if (item["price"].is_string())
              update.price = std::stod(item["price"].get<std::string>());
            else
              update.price = item["price"].get<double>();
          }
          if (update_callback_ && !update.token_id.empty()) {
            update_callback_(update);
          }
        }
        return;
      }

      // ── Fallback: generic object with asset_id + price ──
      if (j.is_object()) {
        if (j.contains("event") && j["event"] == "info")
          return;

        OrderBookUpdate update;
        if (j.contains("asset_id"))
          update.token_id = j["asset_id"].get<std::string>();
        if (j.contains("price")) {
          if (j["price"].is_string())
            update.price = std::stod(j["price"].get<std::string>());
          else
            update.price = j["price"].get<double>();
        }
        if (update_callback_ && !update.token_id.empty())
          update_callback_(update);
      }

    } catch (const std::exception &e) {
      spdlog::warn("JSON Parse Error: {} | Payload: {}", e.what(),
                   msg.str.substr(0, 100));
    }
  } else if (msg.type == ix::WebSocketMessageType::Error) {
    spdlog::error("WebSocket Error: {}", msg.errorInfo.reason);
  }
}
