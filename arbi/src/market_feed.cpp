#include "arbi/market_feed.hpp"
#include <chrono>
#include <curl/curl.h>
#include <iomanip>
#include <mutex>
#include <nlohmann/json.hpp>
#include <openssl/buffer.h>
#include <openssl/evp.h>
#include <openssl/hmac.h>
#include <spdlog/spdlog.h>
#include <sstream>
#include <stdexcept>

using json = nlohmann::json;

namespace arbi {

// ── cURL write callback ──────────────────────────────────────────────
static size_t writeCallback(char *data, size_t size, size_t nmemb,
                            void *userp) {
  auto *buf = static_cast<std::string *>(userp);
  buf->append(data, size * nmemb);
  return size * nmemb;
}

// Thread-safe curl lifecycle management
static std::once_flag curl_init_flag;
static void initCurlOnce() {
  curl_global_init(CURL_GLOBAL_DEFAULT);
  std::atexit(curl_global_cleanup);
}

MarketFeed::MarketFeed(const Config &config) : config_(config) {
  std::call_once(curl_init_flag, initCurlOnce);
}

MarketFeed::~MarketFeed() {
  // cleanup handled by atexit registered in initCurlOnce
}

// ── Authorization Helpers ────────────────────────────────────────────
static std::string base64_encode(const unsigned char *input, int length) {
  BIO *bmem, *b64;
  BUF_MEM *bptr;

  b64 = BIO_new(BIO_f_base64());
  BIO_set_flags(b64, BIO_FLAGS_BASE64_NO_NL); // No newlines
  bmem = BIO_new(BIO_s_mem());
  b64 = BIO_push(b64, bmem);
  BIO_write(b64, input, length);
  BIO_flush(b64);
  BIO_get_mem_ptr(b64, &bptr);

  std::string buff(bptr->data, bptr->length);
  BIO_free_all(b64);

  return buff;
}

static std::string hmac_sha256(const std::string &key,
                               const std::string &data) {
  unsigned char hash[EVP_MAX_MD_SIZE];
  unsigned int len;
  HMAC(EVP_sha256(), key.c_str(), key.length(),
       reinterpret_cast<const unsigned char *>(data.c_str()), data.length(),
       hash, &len);
  return base64_encode(hash, len);
}

// ── HTTP helpers ─────────────────────────────────────────────────────
std::string MarketFeed::httpGet(const std::string &url) {
  CURL *curl = curl_easy_init();
  if (!curl)
    throw std::runtime_error("Failed to init curl");

  std::string response;
  curl_easy_setopt(curl, CURLOPT_URL, url.c_str());
  curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, writeCallback);
  curl_easy_setopt(curl, CURLOPT_WRITEDATA, &response);
  curl_easy_setopt(curl, CURLOPT_TIMEOUT, 10L);
  curl_easy_setopt(curl, CURLOPT_FOLLOWLOCATION, 1L);

  struct curl_slist *headers = nullptr;
  headers = curl_slist_append(headers, "Accept: application/json");
  curl_easy_setopt(curl, CURLOPT_HTTPHEADER, headers);

  CURLcode res = curl_easy_perform(curl);
  curl_slist_free_all(headers);
  curl_easy_cleanup(curl);

  if (res != CURLE_OK) {
    throw std::runtime_error(std::string("HTTP GET failed: ") +
                             curl_easy_strerror(res));
  }
  return response;
}

std::string MarketFeed::httpPost(const std::string &url,
                                 const std::string &body) {
  CURL *curl = curl_easy_init();
  if (!curl)
    throw std::runtime_error("Failed to init curl");

  std::string response;
  curl_easy_setopt(curl, CURLOPT_URL, url.c_str());
  curl_easy_setopt(curl, CURLOPT_POST, 1L);
  curl_easy_setopt(curl, CURLOPT_POSTFIELDS, body.c_str());
  curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, writeCallback);
  curl_easy_setopt(curl, CURLOPT_WRITEDATA, &response);
  curl_easy_setopt(curl, CURLOPT_TIMEOUT, 10L);

  struct curl_slist *headers = nullptr;
  headers = curl_slist_append(headers, "Content-Type: application/json");
  headers = curl_slist_append(headers, "Accept: application/json");

  // Auth headers for live mode (HMAC-SHA256)
  if (config_.live_mode && !config_.polymarket_api_key.empty()) {
    // 1. Timestamp (seconds)
    auto now = std::chrono::system_clock::now();
    auto timestamp = std::to_string(
        std::chrono::duration_cast<std::chrono::seconds>(now.time_since_epoch())
            .count());

    // 2. Extract path (e.g. /order) from URL
    // URL: https://clob.polymarket.com/order -> /order
    std::string path;
    size_t pos = url.find(".com");
    if (pos != std::string::npos) {
      path = url.substr(pos + 4);
    } else {
      path = "/order"; // Fallback
    }

    // 3. Construct payload: timestamp + method + path + body
    std::string method = "POST";
    std::string sig_payload = timestamp + method + path + body;

    // 4. Sign
    std::string signature = hmac_sha256(config_.polymarket_secret, sig_payload);

    // 5. Add Headers
    std::string h_key = "Poly-Api-Key: " + config_.polymarket_api_key;
    std::string h_sig = "Poly-Api-Signature: " + signature;
    std::string h_ts = "Poly-Api-Timestamp: " + timestamp;
    std::string h_pass =
        "Poly-Api-Passphrase: " + config_.polymarket_passphrase;

    headers = curl_slist_append(headers, h_key.c_str());
    headers = curl_slist_append(headers, h_sig.c_str());
    headers = curl_slist_append(headers, h_ts.c_str());
    headers = curl_slist_append(headers, h_pass.c_str());
  }

  curl_easy_setopt(curl, CURLOPT_HTTPHEADER, headers);

  CURLcode res = curl_easy_perform(curl);
  curl_slist_free_all(headers);
  curl_easy_cleanup(curl);

  if (res != CURLE_OK) {
    throw std::runtime_error(std::string("HTTP POST failed: ") +
                             curl_easy_strerror(res));
  }
  return response;
}

// ── Fetch all active markets ─────────────────────────────────────────
std::vector<Market> MarketFeed::fetchMarkets() {
  spdlog::info("[Feed] Fetching active markets...");
  std::vector<Market> markets;

  try {
    // Polymarket Gamma API for market list
    std::string url =
        "https://gamma-api.polymarket.com/markets?closed=false&limit=" +
        std::to_string(config_.max_markets) + "&order=volume&ascending=false";
    auto raw = httpGet(url);
    auto data = json::parse(raw);

    if (!data.is_array()) {
      spdlog::warn("[Feed] Unexpected response format");
      return markets;
    }

    for (auto &m : data) {
      Market market;
      market.condition_id = m.value("conditionId", "");
      market.question = m.value("question", "");
      market.slug = m.value("slug", "");
      market.volume = m.value("volumeNum", 0.0);
      market.category = m.value("category", "");
      market.active = !m.value("closed", false);

      // Token IDs from clobTokenIds (JSON string of array)
      if (m.contains("clobTokenIds")) {
        try {
          auto tokens = json::parse(m["clobTokenIds"].get<std::string>());
          if (tokens.is_array() && tokens.size() >= 2) {
            market.token_id_yes = tokens[0].get<std::string>();
            market.token_id_no = tokens[1].get<std::string>();
          }
        } catch (...) {
        }
      }

      // Prices from outcomePrices
      if (m.contains("outcomePrices")) {
        try {
          auto prices = json::parse(m["outcomePrices"].get<std::string>());
          if (prices.is_array() && prices.size() >= 2) {
            market.yes_price = std::stod(prices[0].get<std::string>());
            market.no_price = std::stod(prices[1].get<std::string>());
          }
        } catch (...) {
        }
      }

      if (!market.condition_id.empty() && market.active) {
        markets.push_back(std::move(market));
      }
    }

    spdlog::info("[Feed] Fetched {} active markets", markets.size());
  } catch (const std::exception &e) {
    spdlog::error("[Feed] Error fetching markets: {}", e.what());
  }

  return markets;
}

// ── Fetch order book ─────────────────────────────────────────────────
OrderBook MarketFeed::fetchOrderBook(const std::string &token_id) {
  OrderBook book;
  book.token_id = token_id;

  try {
    std::string url = baseUrl_ + "/book?token_id=" + token_id;
    auto raw = httpGet(url);
    auto data = json::parse(raw);

    if (data.contains("bids")) {
      for (auto &b : data["bids"]) {
        double price = std::stod(b.value("price", "0"));
        double size = std::stod(b.value("size", "0"));
        book.bids.push_back({price, size});
      }
    }
    if (data.contains("asks")) {
      for (auto &a : data["asks"]) {
        double price = std::stod(a.value("price", "0"));
        double size = std::stod(a.value("size", "0"));
        book.asks.push_back({price, size});
      }
    }

    // Sort: bids descending, asks ascending
    std::sort(book.bids.begin(), book.bids.end(),
              [](auto &a, auto &b) { return a.price > b.price; });
    std::sort(book.asks.begin(), book.asks.end(),
              [](auto &a, auto &b) { return a.price < b.price; });

  } catch (const std::exception &e) {
    spdlog::warn("[Feed] OrderBook fetch failed for {}: {}",
                 token_id.substr(0, 12), e.what());
  }

  return book;
}

// ── Fetch multiple order books ───────────────────────────────────────
std::unordered_map<std::string, OrderBook>
MarketFeed::fetchOrderBooks(const std::vector<std::string> &token_ids) {
  std::unordered_map<std::string, OrderBook> books;
  for (auto &tid : token_ids) {
    books[tid] = fetchOrderBook(tid);
  }
  return books;
}

// ── Submit order ─────────────────────────────────────────────────────
std::optional<std::string> MarketFeed::submitOrder(const std::string &token_id,
                                                   Side side, double price,
                                                   double size) {

  if (!config_.live_mode) {
    // Paper mode — simulate fill
    std::string fake_id =
        "PAPER_" +
        std::to_string(
            std::chrono::steady_clock::now().time_since_epoch().count());
    spdlog::info("[Paper] Order: {} {} @ {:.3f} x {:.2f} → {}",
                 side == Side::BUY ? "BUY" : "SELL", token_id.substr(0, 12),
                 price, size, fake_id);
    return fake_id;
  }

  // Live mode
  try {
    json order_body = {
        {"tokenID", token_id},
        {"side", side == Side::BUY ? "BUY" : "SELL"},
        {"price", std::to_string(price)},
        {"size", std::to_string(size)},
        {"type", "GTC"}, // Good Till Cancelled
    };

    spdlog::info("[Live] Submitting: {} {} @ {:.3f} x {:.2f}",
                 side == Side::BUY ? "BUY" : "SELL", token_id.substr(0, 12),
                 price, size);

    auto response = httpPost(baseUrl_ + "/order", order_body.dump());
    auto resp_json = json::parse(response);

    if (resp_json.contains("orderID")) {
      auto oid = resp_json["orderID"].get<std::string>();
      spdlog::info("[Live] Order placed: {}", oid);
      return oid;
    } else {
      spdlog::error("[Live] Order rejected: {}", response.substr(0, 200));
      return std::nullopt;
    }
  } catch (const std::exception &e) {
    spdlog::error("[Live] Order submission failed: {}", e.what());
    return std::nullopt;
  }
}

} // namespace arbi
