#include "arbi/dependency_graph.hpp"
#include <curl/curl.h>
#include <nlohmann/json.hpp>
#include <spdlog/spdlog.h>
#include <sstream>

using json = nlohmann::json;

namespace arbi {

// cURL callback
static size_t writeCb(char *d, size_t s, size_t n, void *p) {
  static_cast<std::string *>(p)->append(d, s * n);
  return s * n;
}

DependencyGraph::DependencyGraph(const Config &config) : config_(config) {}

// ── Call local LLM via Ollama ────────────────────────────────────────
std::string DependencyGraph::callGroq(const std::string &prompt) {
  CURL *curl = curl_easy_init();
  if (!curl)
    throw std::runtime_error("curl init failed");

  // Use local Ollama (OpenAI-compatible API)
  std::string model = "deepseek-r1:8b";
  json body = {{"model", model},
               {"messages", {{{"role", "user"}, {"content", prompt}}}},
               {"temperature", 0.0},
               {"max_tokens", 2048},
               {"stream", false}};

  std::string response;
  std::string url = "http://localhost:11434/v1/chat/completions";

  curl_easy_setopt(curl, CURLOPT_URL, url.c_str());
  curl_easy_setopt(curl, CURLOPT_POST, 1L);
  std::string body_str = body.dump();
  curl_easy_setopt(curl, CURLOPT_POSTFIELDS, body_str.c_str());
  curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, writeCb);
  curl_easy_setopt(curl, CURLOPT_WRITEDATA, &response);
  curl_easy_setopt(curl, CURLOPT_TIMEOUT, 300L); // increased for R1 thinking

  struct curl_slist *headers = nullptr;
  headers = curl_slist_append(headers, "Content-Type: application/json");
  curl_easy_setopt(curl, CURLOPT_HTTPHEADER, headers);

  CURLcode res = curl_easy_perform(curl);
  curl_slist_free_all(headers);
  curl_easy_cleanup(curl);

  if (res != CURLE_OK) {
    throw std::runtime_error(std::string("Local LLM call failed: ") +
                             curl_easy_strerror(res));
  }

  auto resp = json::parse(response);
  if (resp.contains("choices") && !resp["choices"].empty()) {
    return resp["choices"][0]["message"]["content"].get<std::string>();
  }
  return "";
}

// ── Classify a batch of market pairs ─────────────────────────────────
std::vector<std::pair<std::string, Relation>> DependencyGraph::classifyBatch(
    const std::vector<std::pair<Market, Market>> &pairs) {

  std::vector<std::pair<std::string, Relation>> results;

  // Build prompt for batch classification
  std::ostringstream prompt;
  prompt << "You are a prediction market analyst. For each pair of "
         << "markets below, classify the logical relationship.\n\n"
         << "Respond ONLY with one line per pair in this exact format:\n"
         << "PAIR_INDEX|RELATION\n\n"
         << "Where RELATION is one of:\n"
         << "- IMPLIES (if market A is true, market B must be true)\n"
         << "- MUTEX (markets cannot both be true)\n"
         << "- INDEPENDENT (no logical dependency)\n\n"
         << "Pairs:\n";

  prompt << i << ". A: \"" << pairs[i].first.question << "\" vs B: \""
         << pairs[i].second.question << "\"\n";
}
prompt << "\nFinal Answer:\n";

try {
  auto response = callGroq(prompt.str());

  // Parse response line by line
  std::istringstream stream(response);
  std::string line;
  while (std::getline(stream, line)) {
    // Trim
    while (!line.empty() && (line.back() == '\n' || line.back() == '\r'))
      line.pop_back();

    auto sep = line.find('|');
    if (sep == std::string::npos)
      continue;

    std::string idx_str = line.substr(0, sep);
    std::string rel_str = line.substr(sep + 1);

    // Remove whitespace
    idx_str.erase(0, idx_str.find_first_not_of(" \t"));
    rel_str.erase(0, rel_str.find_first_not_of(" \t"));

    Relation rel = Relation::INDEPENDENT;
    if (rel_str.find("IMPLIES") != std::string::npos)
      rel = Relation::IMPLIES;
    else if (rel_str.find("MUTEX") != std::string::npos)
      rel = Relation::MUTEX;

    try {
      size_t idx = std::stoul(idx_str);
      if (idx < pairs.size()) {
        std::string key = pairs[idx].first.condition_id + ":" +
                          pairs[idx].second.condition_id;
        results.push_back({key, rel});
      }
    } catch (...) {
    }
  }
} catch (const std::exception &e) {
  spdlog::error("[DepGraph] Local LLM classification failed: {}", e.what());
}

return results;
}

// ── Discover dependencies ────────────────────────────────────────────
// ── Get dependencies synchronously (fast) ────────────────────────────
std::vector<Dependency>
DependencyGraph::getDependencies(const std::vector<Market> &markets) {
  std::vector<Dependency> result_deps;

  // Lock cache for read access
  std::lock_guard<std::mutex> lock(cache_mutex_);

  // Reconstruct dependencies based on current market indices
  for (size_t i = 0; i < markets.size(); i++) {
    for (size_t j = i + 1; j < markets.size(); j++) {
      std::string key = markets[i].condition_id + ":" + markets[j].condition_id;
      if (cache_.count(key)) {
        Relation rel = cache_.at(key);
        if (rel != Relation::INDEPENDENT) {
          result_deps.push_back({i, j, rel});
        }
      }
    }
  }
  return result_deps;
}

// ── Start async discovery ────────────────────────────────────────────
void DependencyGraph::startAsyncDiscovery(const std::vector<Market> &markets) {
  if (is_discovering_) {
    spdlog::info("[DepGraph] Discovery already in progress, skipping start.");
    return;
  }

  is_discovering_ = true;
  spdlog::info("[DepGraph] Starting background discovery...");

  // Copy markets to pass to thread
  auto markets_copy = markets;

  discovery_task_ = std::async(std::launch::async, [this, markets_copy]() {
    try {
      spdlog::info("[DepGraph] Background thread running for {} markets",
                   markets_copy.size());

      // 1. Identify pairs NOT in cache
      std::vector<std::pair<Market, Market>> new_pairs;
      size_t max_pairs = 5;

      {
        std::lock_guard<std::mutex> lock(cache_mutex_);
        for (size_t i = 0;
             i < markets_copy.size() && new_pairs.size() < max_pairs; i++) {
          for (size_t j = i + 1;
               j < markets_copy.size() && new_pairs.size() < max_pairs; j++) {

            std::string key = markets_copy[i].condition_id + ":" +
                              markets_copy[j].condition_id;

            // Skip cached
            if (cache_.count(key))
              continue;

            // Heuristic filter (same category)
            if (markets_copy[i].category == markets_copy[j].category) {
              new_pairs.push_back({markets_copy[i], markets_copy[j]});
            }
          }
        }
      }

      if (new_pairs.empty()) {
        spdlog::info("[DepGraph] Background: No new pairs to classify.");
        is_discovering_ = false;
        return;
      }

      // 2. Call LLM (slow, outside lock)
      spdlog::info("[DepGraph] Background: Classifying {} new pairs...",
                   new_pairs.size());
      auto results = classifyBatch(new_pairs);

      // 3. Update Cache
      {
        std::lock_guard<std::mutex> lock(cache_mutex_);
        for (const auto &res : results) {
          cache_[res.first] = res.second;
          if (res.second != Relation::INDEPENDENT) {
            spdlog::info(
                "[DepGraph] Background: Found new dependency: {} -> {}",
                res.first, (int)res.second);
          }
        }
      }
      spdlog::info("[DepGraph] Background discovery complete.");

    } catch (const std::exception &e) {
      spdlog::error("[DepGraph] Background error: {}", e.what());
    }
    is_discovering_ = false;
  });
}

} // namespace arbi
