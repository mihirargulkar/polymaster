#include "arbi/kalshi_market_feed.hpp"
#include <algorithm>
#include <cctype>
#include <cmath>
#include <curl/curl.h>
#include <nlohmann/json.hpp>
#include <openssl/bio.h>
#include <openssl/buffer.h>
#include <openssl/evp.h>
#include <openssl/pem.h>
#include <set>
#include <spdlog/spdlog.h>
#include <sstream>

using json = nlohmann::json;

namespace arbi {

// ── cURL write callback ──────────────────────────────────────────────
static size_t writeCallback(char *data, size_t size, size_t nmemb,
                            void *userp) {
  auto *buf = static_cast<std::string *>(userp);
  buf->append(data, size * nmemb);
  return size * nmemb;
}

KalshiMarketFeed::KalshiMarketFeed(const std::string &key_id,
                                   const std::string &private_key_path)
    : key_id_(key_id), private_key_path_(private_key_path) {
  // Load RSA private key
  FILE *fp = fopen(private_key_path.c_str(), "r");
  if (!fp) {
    spdlog::error("[KalshiMarketFeed] Cannot open private key: {}",
                  private_key_path);
    return;
  }
  pkey_ = PEM_read_PrivateKey(fp, nullptr, nullptr, nullptr);
  fclose(fp);
  if (!pkey_) {
    spdlog::error("[KalshiMarketFeed] Failed to parse private key");
  }
}

KalshiMarketFeed::~KalshiMarketFeed() {
  if (pkey_) {
    EVP_PKEY_free(static_cast<EVP_PKEY *>(pkey_));
  }
}

// ── Base64 encoding ──────────────────────────────────────────────────
std::string KalshiMarketFeed::base64Encode(const unsigned char *buffer,
                                           size_t length) {
  BIO *bmem, *b64;
  BUF_MEM *bptr;

  b64 = BIO_new(BIO_f_base64());
  BIO_set_flags(b64, BIO_FLAGS_BASE64_NO_NL);
  bmem = BIO_new(BIO_s_mem());
  b64 = BIO_push(b64, bmem);
  BIO_write(b64, buffer, (int)length);
  BIO_flush(b64);
  BIO_get_mem_ptr(b64, &bptr);

  std::string result(bptr->data, bptr->length);
  BIO_free_all(b64);
  return result;
}

// ── RSA-PSS signing ─────────────────────────────────────────────────
std::string KalshiMarketFeed::signRequest(const std::string &timestamp,
                                          const std::string &method,
                                          const std::string &path) {
  if (!pkey_)
    return "";

  // Kalshi signs: timestamp + method + path
  std::string message = timestamp + method + path;

  EVP_MD_CTX *ctx = EVP_MD_CTX_new();
  EVP_PKEY_CTX *pctx = nullptr;

  if (EVP_DigestSignInit(ctx, &pctx, EVP_sha256(), nullptr,
                         static_cast<EVP_PKEY *>(pkey_)) != 1) {
    EVP_MD_CTX_free(ctx);
    return "";
  }

  EVP_PKEY_CTX_set_rsa_padding(pctx, RSA_PKCS1_PSS_PADDING);
  EVP_PKEY_CTX_set_rsa_pss_saltlen(pctx, EVP_MD_CTX_get_size(ctx));

  if (EVP_DigestSignUpdate(ctx, message.c_str(), message.size()) != 1) {
    EVP_MD_CTX_free(ctx);
    return "";
  }

  size_t sig_len = 0;
  EVP_DigestSignFinal(ctx, nullptr, &sig_len);
  std::vector<unsigned char> sig(sig_len);
  EVP_DigestSignFinal(ctx, sig.data(), &sig_len);
  EVP_MD_CTX_free(ctx);

  return base64Encode(sig.data(), sig_len);
}

// ── Authenticated HTTP GET ───────────────────────────────────────────
std::string KalshiMarketFeed::httpGet(const std::string &url) {
  CURL *curl = curl_easy_init();
  if (!curl)
    throw std::runtime_error("Failed to init curl");

  std::string response;
  curl_easy_setopt(curl, CURLOPT_URL, url.c_str());
  curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, writeCallback);
  curl_easy_setopt(curl, CURLOPT_WRITEDATA, &response);
  curl_easy_setopt(curl, CURLOPT_TIMEOUT, 15L);
  curl_easy_setopt(curl, CURLOPT_FOLLOWLOCATION, 1L);

  // Auth headers
  auto now = std::chrono::system_clock::now();
  auto ts = std::to_string(
      std::chrono::duration_cast<std::chrono::seconds>(now.time_since_epoch())
          .count());

  // Extract path from URL
  std::string path;
  size_t pos = url.find(".com");
  if (pos != std::string::npos)
    path = url.substr(pos + 4);
  else
    path = "/trade-api/v2/events";

  std::string sig = signRequest(ts, "GET", path);

  struct curl_slist *headers = nullptr;
  headers = curl_slist_append(headers, "Accept: application/json");
  headers =
      curl_slist_append(headers, ("KALSHI-ACCESS-KEY: " + key_id_).c_str());
  headers =
      curl_slist_append(headers, ("KALSHI-ACCESS-SIGNATURE: " + sig).c_str());
  headers =
      curl_slist_append(headers, ("KALSHI-ACCESS-TIMESTAMP: " + ts).c_str());
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

// ── Authenticated HTTP POST ──────────────────────────────────────────
std::string KalshiMarketFeed::httpPost(const std::string &url,
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
  curl_easy_setopt(curl, CURLOPT_TIMEOUT, 15L);

  // Auth headers
  auto now = std::chrono::system_clock::now();
  auto ts = std::to_string(
      std::chrono::duration_cast<std::chrono::seconds>(now.time_since_epoch())
          .count());

  // Extract path from URL
  std::string path;
  size_t pos = url.find(".com");
  if (pos != std::string::npos)
    path = url.substr(pos + 4);
  else
    path = "/trade-api/v2/portfolio/orders";

  std::string sig = signRequest(ts, "POST", path);

  struct curl_slist *headers = nullptr;
  headers = curl_slist_append(headers, "Accept: application/json");
  headers = curl_slist_append(headers, "Content-Type: application/json");
  headers =
      curl_slist_append(headers, ("KALSHI-ACCESS-KEY: " + key_id_).c_str());
  headers =
      curl_slist_append(headers, ("KALSHI-ACCESS-SIGNATURE: " + sig).c_str());
  headers =
      curl_slist_append(headers, ("KALSHI-ACCESS-TIMESTAMP: " + ts).c_str());
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

// ── Fetch Kalshi markets ─────────────────────────────────────────────
std::vector<Market> KalshiMarketFeed::fetchMarkets(int limit) {
  spdlog::info("[KalshiMarketFeed] Fetching open events with markets...");
  std::vector<Market> markets;

  try {
    std::string url = "https://api.elections.kalshi.com/trade-api/v2/events"
                      "?status=open&with_nested_markets=true&limit=" +
                      std::to_string(limit);

    auto raw = httpGet(url);
    auto data = json::parse(raw);

    if (!data.contains("events") || !data["events"].is_array()) {
      spdlog::warn("[KalshiMarketFeed] Unexpected response format");
      return markets;
    }

    for (const auto &event : data["events"]) {
      std::string event_ticker = event.value("event_ticker", "");
      std::string event_title = event.value("title", "");

      if (!event.contains("markets") || !event["markets"].is_array())
        continue;

      for (const auto &mkt : event["markets"]) {
        Market m;
        m.exchange = Exchange::KALSHI;
        m.kalshi_ticker = mkt.value("ticker", "");
        m.event_ticker = event_ticker;
        m.question = mkt.value("title", event_title);
        m.slug = mkt.value("ticker", "");
        m.condition_id = m.kalshi_ticker; // use ticker as unique ID

        // Prices: yes_bid / yes_ask (integer cents → dollars)
        double yes_bid = mkt.value("yes_bid", 0) / 100.0;
        double yes_ask = mkt.value("yes_ask", 0) / 100.0;
        m.yes_price = (yes_bid + yes_ask) / 2.0; // midpoint
        m.no_price = 1.0 - m.yes_price;

        // Volume
        m.volume = mkt.value("volume", 0.0);

        // Active check — Kalshi uses "active" not "open"
        std::string status = mkt.value("status", "");
        m.active = (status == "active");

        if (m.active && !m.kalshi_ticker.empty()) {
          markets.push_back(std::move(m));
        }
      }
    }

    spdlog::info("[KalshiMarketFeed] Fetched {} active Kalshi markets",
                 markets.size());
  } catch (const std::exception &e) {
    spdlog::error("[KalshiMarketFeed] Error: {}", e.what());
  }

  return markets;
}

// ── Submit Kalshi Order ─────────────────────────────────────────────
std::optional<std::string>
KalshiMarketFeed::submitOrder(const std::string &ticker, Side side,
                              double price, double count) {
  spdlog::info("[KalshiMarketFeed] Submitting order: {} {} {} @ {:.3f}",
               side == Side::BUY ? "BUY" : "SELL", count, ticker, price);

  try {
    std::string url =
        "https://api.elections.kalshi.com/trade-api/v2/portfolio/orders";

    // Convert price to cents (integer)
    int price_cents = static_cast<int>(std::round(price * 100.0));

    json body = {
        {"ticker", ticker},
        {"action", "buy"}, // We always use "buy" action for Yes/No contracts
        {"type", "limit"},
        {"side", (side == Side::BUY ? "yes" : "no")},
        {"count", static_cast<int>(count)},
        {"client_order_id",
         "arbi_" +
             std::to_string(
                 std::chrono::system_clock::now().time_since_epoch().count())}};

    if (side == Side::BUY) {
      body["yes_price"] = price_cents;
    } else {
      body["no_price"] =
          100 - price_cents; // Sell Yes @ price == Buy No @ (1-price)
    }

    auto raw = httpPost(url, body.dump());
    auto data = json::parse(raw);

    if (data.contains("order_id")) {
      std::string order_id = data["order_id"];
      spdlog::info("[KalshiMarketFeed] Order submitted successfully: {}",
                   order_id);
      return order_id;
    } else {
      spdlog::error("[KalshiMarketFeed] Order submission failed: {}", raw);
      return std::nullopt;
    }
  } catch (const std::exception &e) {
    spdlog::error("[KalshiMarketFeed] Order error: {}", e.what());
    return std::nullopt;
  }
}

// ── Text tokenization ────────────────────────────────────────────────
std::vector<std::string> KalshiMarketFeed::tokenize(const std::string &text) {
  std::vector<std::string> tokens;
  std::string word;

  for (char c : text) {
    if (std::isalnum(c)) {
      word += std::tolower(c);
    } else if (!word.empty()) {
      // Skip common stop words
      if (word.length() > 2 && word != "the" && word != "will" &&
          word != "for" && word != "and" && word != "that" && word != "this" &&
          word != "with" && word != "from" && word != "are" && word != "was" &&
          word != "has" && word != "been" && word != "its" && word != "what") {
        tokens.push_back(word);
      }
      word.clear();
    }
  }
  if (!word.empty() && word.length() > 2) {
    tokens.push_back(word);
  }
  return tokens;
}

// ── Jaccard similarity ──────────────────────────────────────────────
double KalshiMarketFeed::jaccardSimilarity(const std::vector<std::string> &a,
                                           const std::vector<std::string> &b) {
  std::set<std::string> setA(a.begin(), a.end());
  std::set<std::string> setB(b.begin(), b.end());

  size_t intersection = 0;
  for (const auto &word : setA) {
    if (setB.count(word))
      intersection++;
  }

  size_t uni = setA.size() + setB.size() - intersection;
  return uni == 0 ? 0.0 : static_cast<double>(intersection) / uni;
}

// ── Match markets across exchanges ──────────────────────────────────
std::vector<CrossExchangePair>
KalshiMarketFeed::matchMarkets(const std::vector<Market> &poly_markets,
                               const std::vector<Market> &kalshi_markets,
                               double min_similarity) {

  std::vector<CrossExchangePair> pairs;

  // Pre-tokenize all markets
  std::vector<std::vector<std::string>> poly_tokens(poly_markets.size());
  std::vector<std::vector<std::string>> kalshi_tokens(kalshi_markets.size());

  for (size_t i = 0; i < poly_markets.size(); i++)
    poly_tokens[i] = tokenize(poly_markets[i].question);
  for (size_t j = 0; j < kalshi_markets.size(); j++)
    kalshi_tokens[j] = tokenize(kalshi_markets[j].question);

  // O(n*m) pairwise comparison — acceptable for ~200 x ~200
  for (size_t i = 0; i < poly_markets.size(); i++) {
    double best_sim = 0.0;
    size_t best_j = 0;

    for (size_t j = 0; j < kalshi_markets.size(); j++) {
      double sim = jaccardSimilarity(poly_tokens[i], kalshi_tokens[j]);
      if (sim > best_sim) {
        best_sim = sim;
        best_j = j;
      }
    }

    if (best_sim >= min_similarity) {
      CrossExchangePair pair;
      pair.poly_idx = i;
      pair.kalshi_idx = best_j;
      pair.similarity = best_sim;
      pair.poly_yes = poly_markets[i].yes_price;
      pair.kalshi_yes = kalshi_markets[best_j].yes_price;
      pair.spread = std::abs(pair.poly_yes - pair.kalshi_yes);
      pairs.push_back(pair);
    }
  }

  // Sort by spread descending (most profitable first)
  std::sort(pairs.begin(), pairs.end(),
            [](const auto &a, const auto &b) { return a.spread > b.spread; });

  return pairs;
}

} // namespace arbi
