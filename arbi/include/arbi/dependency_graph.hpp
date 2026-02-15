#pragma once
#include "arbi/common.hpp"
#include <atomic>
#include <future>
#include <mutex>
#include <unordered_map>

namespace arbi {

class DependencyGraph {
public:
  explicit DependencyGraph(const Config &config);

  // Discover dependencies between markets using Groq LLM
  std::vector<Dependency> discover(const std::vector<Market> &markets);

  // Get cached dependencies (avoid re-querying)
  const std::vector<Dependency> &cached() const { return deps_; }

  // Clear cache (force re-discovery)
  void clearCache() {
    std::lock_guard<std::mutex> lock(cache_mutex_);
    deps_.clear();
    cache_.clear();
  }

  // Start background discovery (non-blocking)
  void startAsyncDiscovery(const std::vector<Market> &markets);

  // Get current dependencies synchronously (fast)
  std::vector<Dependency> getDependencies(const std::vector<Market> &markets);

private:
  Config config_;
  std::vector<Dependency> deps_;

  // Thread safety
  mutable std::mutex cache_mutex_;
  std::atomic<bool> is_discovering_{false};
  std::future<void> discovery_task_;

  // Cache: "i:j" -> Relation
  std::unordered_map<std::string, Relation> cache_;

  // Call Groq API to classify a batch of market pairs
  std::vector<std::pair<std::string, Relation>>
  classifyBatch(const std::vector<std::pair<Market, Market>> &pairs);

  std::string callGroq(const std::string &prompt);
};

} // namespace arbi
