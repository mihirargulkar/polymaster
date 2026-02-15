cmake_minimum_required(VERSION 3.20)
project(arbi VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# ── Homebrew prefix (arm64 macOS) ─────────────────────────────────────
set(CMAKE_PREFIX_PATH "/opt/homebrew" ${CMAKE_PREFIX_PATH})

# ── Dependencies (all via Homebrew) ───────────────────────────────────
find_package(nlohmann_json REQUIRED)
find_package(Eigen3 REQUIRED)
find_package(spdlog REQUIRED)
find_package(CURL REQUIRED)
find_package(OpenSSL REQUIRED)
find_package(ZLIB REQUIRED)

set(USE_TLS 1)
set(USE_OPEN_SSL 1)

include(FetchContent)
FetchContent_Declare(
  ixwebsocket
  GIT_REPOSITORY https://github.com/machinezone/IXWebSocket.git
  GIT_TAG        v11.0.8
)
FetchContent_MakeAvailable(ixwebsocket)

# GLPK (no cmake config, use manual find)
find_library(GLPK_LIB glpk HINTS /opt/homebrew/lib /usr/local/lib)
find_path(GLPK_INCLUDE glpk.h HINTS /opt/homebrew/include /usr/local/include)
if(NOT GLPK_LIB OR NOT GLPK_INCLUDE)
  message(FATAL_ERROR "GLPK not found. Install with: brew install glpk")
endif()

# ── Arbi executable ──────────────────────────────────────────────────
add_executable(arbi
  src/main.cpp
  src/market_feed.cpp
  src/websocket_feed.cpp
  src/kalshi_feed.cpp
  src/kalshi_market_feed.cpp
  src/dependency_graph.cpp
  src/polytope.cpp
  src/bregman.cpp
  src/frank_wolfe.cpp
  src/execution.cpp
  src/cross_exchange_execution.cpp
  src/logger.cpp
)

target_include_directories(arbi PRIVATE
  ${CMAKE_SOURCE_DIR}/include
  ${GLPK_INCLUDE}
)

target_link_libraries(arbi PRIVATE
  nlohmann_json::nlohmann_json
  Eigen3::Eigen
  spdlog::spdlog
  ${GLPK_LIB}
  CURL::libcurl
  OpenSSL::SSL
  OpenSSL::Crypto
  ixwebsocket
)

# ── Test binary ───────────────────────────────────────────────────────
add_executable(arbi_tests
  tests/test_all.cpp
  src/polytope.cpp
  src/bregman.cpp
  src/frank_wolfe.cpp
  src/logger.cpp
)

target_include_directories(arbi_tests PRIVATE
  ${CMAKE_SOURCE_DIR}/include
  ${GLPK_INCLUDE}
)

target_link_libraries(arbi_tests PRIVATE
  Eigen3::Eigen
  spdlog::spdlog
  ${GLPK_LIB}
)

add_executable(test_ws_connectivity tests/test_ws_connectivity.cpp)
target_link_libraries(test_ws_connectivity PRIVATE 
  ixwebsocket
  "-framework Security"
  "-framework CoreFoundation"
  "-framework SystemConfiguration"
  ZLIB::ZLIB
)

add_executable(test_dns tests/test_dns.cpp)
target_link_libraries(test_dns PRIVATE 
  "-framework CoreFoundation" 
  "-framework SystemConfiguration"
  resolv
)
