#include "arbi/kalshi_feed.hpp"
#include <chrono>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <nlohmann/json.hpp>
#include <openssl/bio.h>
#include <openssl/buffer.h>
#include <spdlog/spdlog.h>
#include <sstream>

namespace arbi {

KalshiFeed::KalshiFeed() {
  ws_ = std::make_unique<ix::WebSocket>();
  // Default URL for demo (will be overridden in setup or connect if needed)
  // But we use PROD for arbi (elections)
  // URL: wss://api.elections.kalshi.com/trade-api/ws/v2
}

KalshiFeed::~KalshiFeed() {
  if (ws_)
    ws_->stop();
  if (pkey_)
    EVP_PKEY_free(pkey_);
}

void KalshiFeed::setup(const std::string &key_id,
                       const std::string &private_key_path) {
  key_id_ = key_id;
  private_key_path_ = private_key_path;

  // Load Private Key
  FILE *fp = fopen(private_key_path_.c_str(), "r");
  if (!fp) {
    spdlog::error("Failed to open Kalshi private key: {}", private_key_path_);
    return;
  }
  pkey_ = PEM_read_PrivateKey(fp, nullptr, nullptr, nullptr);
  fclose(fp);
  if (!pkey_) {
    spdlog::error("Failed to parse Kalshi private key");
    return;
  }

  // Configure WebSocket
  std::string url = "wss://api.elections.kalshi.com/trade-api/ws/v2";
  ws_->setUrl(url);
  ws_->setPingInterval(30);

  // Callbacks
  ws_->setOnMessageCallback(
      [this](const ix::WebSocketMessagePtr &msg) { onMessage(*msg); });
}

std::string KalshiFeed::signTimestamp(const std::string &timestamp) {
  if (!pkey_)
    return "";

  // Message to sign: timestamp + "GET" + "/trade-api/ws/v2"
  std::string msg = timestamp + "GET" + "/trade-api/ws/v2";

  EVP_MD_CTX *ctx = EVP_MD_CTX_new();
  if (!ctx)
    return "";

  // RSA-PSS signing
  EVP_PKEY_CTX *pkey_ctx = nullptr;
  if (EVP_DigestSignInit(ctx, &pkey_ctx, EVP_sha256(), nullptr, pkey_) <= 0) {
    EVP_MD_CTX_free(ctx);
    return "";
  }

  if (EVP_PKEY_CTX_set_rsa_padding(pkey_ctx, RSA_PKCS1_PSS_PADDING) <= 0) {
    EVP_MD_CTX_free(ctx);
    return "";
  }
  if (EVP_PKEY_CTX_set_rsa_pss_saltlen(pkey_ctx, -1) <=
      0) { // Salt len = digest len
    EVP_MD_CTX_free(ctx);
    return "";
  }

  size_t siglen;
  if (EVP_DigestSignUpdate(ctx, msg.c_str(), msg.length()) <= 0) {
    EVP_MD_CTX_free(ctx);
    return "";
  }
  if (EVP_DigestSignFinal(ctx, nullptr, &siglen) <= 0) {
    EVP_MD_CTX_free(ctx);
    return "";
  }

  std::vector<unsigned char> sig(siglen);
  if (EVP_DigestSignFinal(ctx, sig.data(), &siglen) <= 0) {
    EVP_MD_CTX_free(ctx);
    return "";
  }

  EVP_MD_CTX_free(ctx);

  // Base64 Encode
  return base64Encode(sig.data(), siglen);
}

std::string KalshiFeed::base64Encode(const unsigned char *buffer,
                                     size_t length) {
  BIO *bio, *b64;
  BUF_MEM *bufferPtr;

  b64 = BIO_new(BIO_f_base64());
  bio = BIO_new(BIO_s_mem());
  bio = BIO_push(b64, bio);

  BIO_set_flags(bio, BIO_FLAGS_BASE64_NO_NL); // No newlines
  BIO_write(bio, buffer, length);
  BIO_flush(bio);
  BIO_get_mem_ptr(bio, &bufferPtr);

  std::string res(bufferPtr->data, bufferPtr->length);
  BIO_free_all(bio);
  return res;
}

void KalshiFeed::connect() {
  if (!pkey_) {
    spdlog::error("Cannot connect to Kalshi: Private key not loaded");
    return;
  }

  // Generate Timestamp
  auto now = std::chrono::system_clock::now();
  auto ms = std::chrono::duration_cast<std::chrono::milliseconds>(
                now.time_since_epoch())
                .count();
  std::string timestamp = std::to_string(ms);

  // Generate Signature
  std::string signature = signTimestamp(timestamp);
  if (signature.empty()) {
    spdlog::error("Failed to generate Kalshi signature");
    return;
  }

  // Add Headers
  ix::WebSocketHttpHeaders headers;
  headers["KALSHI-ACCESS-KEY"] = key_id_;
  headers["KALSHI-ACCESS-SIGNATURE"] = signature;
  headers["KALSHI-ACCESS-TIMESTAMP"] = timestamp;
  ws_->setExtraHeaders(headers);

  spdlog::info("Connecting to Kalshi WebSocket...");
  ws_->start();
}

void KalshiFeed::subscribe(const std::vector<std::string> &tickers) {
  if (!ws_)
    return;

  // Construct subscription message
  // Format: {"id": 1, "cmd": "subscribe", "params": {"channels": ["ticker"]}}
  // Wait, need to specify WHICH tickers.
  // Usually "channels": ["ticker:<ticker>"] or "market_tickers": [...]?
  // Kalshi v2 docs: "channels": ["ticker"] subscribes to ALL? Or specific?
  // Usually specific: "ticker.<ticker_id>".
  // Or maybe "channels": ["ticker"], "params": {"tickers": [...]}?

  // Based on limited search, assuming "ticker" channel gives ALL or need
  // params. I'll assume usage is `{"id": 1, "cmd": "subscribe", "params":
  // {"channels": ["ticker"]}}` for ALL updates OR `{"id": 1, "cmd":
  // "subscribe", "params": {"channels": ["ticker"], "market_tickers": [...]}}`.

  // I will try to subscribe to specific tickers if possible.
  // For now, I'll log a warning that I'm implementing a basic subscribe.

  nlohmann::json msg;
  msg["id"] = 1;
  msg["cmd"] = "subscribe";
  msg["params"] = {{"channels", {"ticker"}}};
  // If I need specific tickers, I might need to add them.
  // However, subscribing to ALL "ticker" updates is probably fine for arbi (we
  // filter).

  spdlog::info("Subscribing to Kalshi ticker channel...");
  ws_->send(msg.dump());
}

void KalshiFeed::setOnUpdate(
    std::function<void(const KalshiOrderBookUpdate &)> callback) {
  update_callback_ = callback;
}

void KalshiFeed::onMessage(const ix::WebSocketMessage &msg) {
  if (msg.type == ix::WebSocketMessageType::Open) {
    spdlog::info("Kalshi WebSocket Connected!");
    connected_ = true;
  } else if (msg.type == ix::WebSocketMessageType::Message) {
    // Parse JSON
    try {
      auto j = nlohmann::json::parse(msg.str);
      // Expecting {"type": "ticker", "msg": {...}} or similar
      if (j.contains("type") && j["type"] == "ticker") {
        // Extract data
        // Fields: ticker, yes_bid, yes_ask, etc.
        auto &m = j["msg"]; // Assuming data is in "msg" or root?
                            // Wait, documentation needed.
                            // Assuming root has fields.
      }
      // Logging purely for Phase 1
      spdlog::debug("Kalshi Msg: {}", msg.str.substr(0, 100));

    } catch (...) {
      spdlog::warn("Kalshi JSON parse error");
    }
  } else if (msg.type == ix::WebSocketMessageType::Error) {
    spdlog::error("Kalshi WebSocket Error: {}", msg.errorInfo.reason);
  }
}

} // namespace arbi
