#include <chrono>
#include <iostream>
#include <ixwebsocket/IXNetSystem.h>
#include <ixwebsocket/IXUserAgent.h>
#include <ixwebsocket/IXWebSocket.h>
#include <thread>

int main() {
  ix::initNetSystem();

  std::string url = "wss://ws-subscriptions-clob.polymarket.com/ws/market";

  ix::WebSocket webSocket;
  webSocket.setUrl(url);

  std::cout << "Connecting to " << url << "..." << std::endl;

  webSocket.setOnMessageCallback([](const ix::WebSocketMessagePtr &msg) {
    if (msg->type == ix::WebSocketMessageType::Open) {
      std::cout << "[WS] Connection established!" << std::endl;
    } else if (msg->type == ix::WebSocketMessageType::Message) {
      std::cout << "[WS] Received message: " << msg->str << std::endl;
    } else if (msg->type == ix::WebSocketMessageType::Error) {
      std::cout << "[WS] Error: " << msg->errorInfo.reason << std::endl;
    }
  });

  webSocket.start();
  std::this_thread::sleep_for(std::chrono::seconds(5));
  webSocket.stop();
  return 0;
}
