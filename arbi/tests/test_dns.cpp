#include <arpa/inet.h>
#include <iostream>
#include <netdb.h>
#include <netinet/in.h>
#include <resolv.h>
#include <string.h>

int main() {
  struct addrinfo hints, *res;
  int errcode;

  res_init();

  // memset(&hints, 0, sizeof(hints));
  // hints.ai_family = AF_UNSPEC;
  // hints.ai_socktype = SOCK_STREAM;

  errcode = getaddrinfo("ws-gamma-clob.polymarket.com", NULL, NULL, &res);
  if (errcode != 0) {
    std::cout << "getaddrinfo(polymarket) FAILED: " << gai_strerror(errcode)
              << std::endl;
    return 1;
  }

  std::cout << "getaddrinfo SUCCESS" << std::endl;
  freeaddrinfo(res);
  return 0;
}
