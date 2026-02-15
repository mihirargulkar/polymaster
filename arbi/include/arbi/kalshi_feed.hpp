#pragma once

#include "arbi/common.hpp"
#include <functional>
#include <ixwebsocket/IXWebSocket.h>
#include <ixwebsocket/IXWebSocketMessage.h>
#include <memory>
#include <openssl/err.h>
#include <openssl/evp.h>
#include <openssl/pem.h>
#include <string>
#include <vector>

namespace arbi {

struct KalshiOrderBookUpdate {
  std::string ticker;
  double timestamp;
  double best_bid;
  double best_ask;
};

class KalshiFeed {
public:
  KalshiFeed();
  ~KalshiFeed();

  void setup(const std::string &key_id, const std::string &private_key_path);
  void connect();
  void subscribe(const std::vector<std::string> &tickers);
  void setOnUpdate(std::function<void(const KalshiOrderBookUpdate &)> callback);

private:
  void onMessage(const ix::WebSocketMessage &msg);
  std::string signTimestamp(const std::string &timestamp);
  std::string base64Encode(const unsigned char *buffer, size_t length);

  std::unique_ptr<ix::WebSocket> ws_;
  std::function<void(const KalshiOrderBookUpdate &)> update_callback_;
  bool connected_ = false;
  std::string key_id_;
  std::string private_key_path_;
  EVP_PKEY *pkey_ = nullptr;
};

} // namespace arbi
