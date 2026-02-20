use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

const DEFAULT_TOP_K: usize = 15;
const CACHE_TTL_SECS: u64 = 3600;

// ── Types ───────────────────────────────────────────────────────────────

pub struct MarketMatcher {
    client: reqwest::Client,
    model: String,
    embed_model: String,
    generate_url: String,
    embed_url: String,
    embedding_index: HashMap<String, Vec<f32>>,
    match_cache: HashMap<u64, (MatchResult, Instant)>,
    cache_ttl: Duration,
}

#[derive(Deserialize, Debug)]
struct OllamaResponse {
    response: String,
}

#[derive(Deserialize)]
struct OllamaEmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MatchResult {
    pub r#match: bool,
    #[serde(default)]
    pub ticker: String,
    #[serde(default)]
    pub side: String,
    pub confidence: Option<f64>,
    pub reasoning: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidenceTier {
    Exact,   // >= 0.95
    Related, // 0.80 – 0.95
    None,    // < 0.80
}

// ── Pure functions (testable) ───────────────────────────────────────────

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a < f32::EPSILON || norm_b < f32::EPSILON {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

pub fn confidence_tier(confidence: f64) -> ConfidenceTier {
    if confidence >= 0.95 {
        ConfidenceTier::Exact
    } else if confidence >= 0.80 {
        ConfidenceTier::Related
    } else {
        ConfidenceTier::None
    }
}

static STOP_WORDS: &[&str] = &[
    "will", "the", "and", "for", "that", "this", "with", "from", "are",
    "was", "been", "have", "has", "had", "its", "but", "not", "they",
    "you", "price", "market", "outcome",
];

static EXPANSIONS: &[(&str, &[&str])] = &[
    ("btc", &["bitcoin"]),
    ("bitcoin", &["btc"]),
    ("eth", &["ethereum"]),
    ("ethereum", &["eth"]),
    ("fed", &["federal", "reserve"]),
    ("gop", &["republican"]),
    ("dem", &["democrat", "democratic"]),
    ("scotus", &["supreme", "court"]),
    ("potus", &["president"]),
    ("nba", &["basketball"]),
    ("nfl", &["football"]),
    ("mlb", &["baseball"]),
    ("nhl", &["hockey"]),
    ("o/u", &["over", "under"]),
    ("fc", &["football", "club"]),
    // NBA team ↔ city mappings (Polymarket uses team names, Kalshi uses cities)
    ("hawks", &["atlanta"]),
    ("celtics", &["boston", "bos"]),
    ("nets", &["brooklyn"]),
    ("hornets", &["charlotte", "cha"]),
    ("bulls", &["chicago"]),
    ("cavaliers", &["cleveland"]),
    ("cavs", &["cleveland"]),
    ("mavericks", &["dallas", "dal"]),
    ("mavs", &["dallas", "dal"]),
    ("nuggets", &["denver", "den"]),
    ("pistons", &["detroit", "det"]),
    ("warriors", &["golden", "state", "gsw"]),
    ("rockets", &["houston", "hou"]),
    ("pacers", &["indiana", "ind"]),
    ("clippers", &["los", "angeles", "lac"]),
    ("lakers", &["los", "angeles", "lal"]),
    ("grizzlies", &["memphis", "mem"]),
    ("heat", &["miami", "mia"]),
    ("bucks", &["milwaukee", "mil"]),
    ("timberwolves", &["minnesota", "min"]),
    ("pelicans", &["new", "orleans", "nop"]),
    ("knicks", &["new", "york", "nyk"]),
    ("thunder", &["oklahoma", "city", "okc"]),
    ("magic", &["orlando", "orl"]),
    ("76ers", &["philadelphia", "phi"]),
    ("sixers", &["philadelphia", "phi"]),
    ("suns", &["phoenix", "phx"]),
    ("blazers", &["portland", "por"]),
    ("kings", &["sacramento", "sac"]),
    ("spurs", &["san", "antonio", "sas"]),
    ("raptors", &["toronto", "tor"]),
    ("jazz", &["utah", "uta"]),
    ("wizards", &["washington", "was"]),
];

/// Expand common prediction-market abbreviations and remove stop words.
pub fn expand_keywords(title: &str) -> Vec<String> {
    let mut words: Vec<String> = title
        .to_lowercase()
        .split_whitespace()
        .filter(|w| w.len() > 2)
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|w| !w.is_empty())
        .collect();

    let originals = words.clone();
    for word in &originals {
        for &(abbr, expansions) in EXPANSIONS {
            if word == abbr {
                for exp in expansions {
                    if !words.contains(&exp.to_string()) {
                        words.push(exp.to_string());
                    }
                }
            }
        }
    }

    words.retain(|w| !STOP_WORDS.contains(&w.as_str()));
    words
}

/// Strip markdown code fences that Ollama sometimes wraps around JSON.
pub fn strip_json_fences(raw: &str) -> &str {
    let trimmed = raw.trim();
    trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|s| s.strip_suffix("```").map(|inner| inner.trim()))
        .unwrap_or(trimmed)
}

fn cache_key(title: &str, outcome: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    title.hash(&mut hasher);
    outcome.hash(&mut hasher);
    hasher.finish()
}

// ── MarketMatcher ───────────────────────────────────────────────────────

impl MarketMatcher {
    /// `model`: LLM for reranking (e.g. "llama3").
    /// `embed_model`: embedding model for retrieval (e.g. "nomic-embed-text").
    /// `base_url`: Ollama server, e.g. "http://localhost:11434".
    pub fn new(model: String, embed_model: String, base_url: Option<&str>) -> Self {
        let base = base_url
            .unwrap_or("http://localhost:11434")
            .trim_end_matches('/');
        Self {
            client: reqwest::Client::new(),
            model,
            embed_model,
            generate_url: format!("{}/api/generate", base),
            embed_url: format!("{}/api/embed", base),
            embedding_index: HashMap::new(),
            match_cache: HashMap::new(),
            cache_ttl: Duration::from_secs(CACHE_TTL_SECS),
        }
    }

    // ── Index management (used by tests, optional for on-demand search) ──

    #[allow(dead_code)]
    pub async fn build_index(&mut self, markets: &[crate::platforms::kalshi::MarketInfo]) {
        let to_embed: Vec<(String, String)> = markets
            .iter()
            .filter(|m| !self.embedding_index.contains_key(&m.ticker))
            .map(|m| (m.ticker.clone(), m.title.clone()))
            .collect();

        if to_embed.is_empty() {
            return;
        }

        println!(
            "📐 Building embedding index for {} new markets...",
            to_embed.len()
        );

        for chunk in to_embed.chunks(50) {
            // nomic-embed-text uses "search_document: " prefix for documents
            let prefixed: Vec<String> = chunk
                .iter()
                .map(|(_, t)| format!("search_document: {}", t))
                .collect();
            let titles: Vec<&str> = prefixed.iter().map(|s| s.as_str()).collect();
            match self.embed_batch(&titles).await {
                Some(embeddings) => {
                    for ((ticker, _), emb) in chunk.iter().zip(embeddings) {
                        self.embedding_index.insert(ticker.clone(), emb);
                    }
                }
                None => {
                    println!("⚠️ Embedding batch failed; will use keyword filter as fallback");
                    return;
                }
            }
        }

        println!(
            "📐 Embedding index ready: {} markets indexed",
            self.embedding_index.len()
        );
    }

    #[allow(dead_code)]
    pub fn clear_index(&mut self) {
        self.embedding_index.clear();
    }

    /// Remove expired entries from the match cache.
    pub fn prune_cache(&mut self) {
        let now = Instant::now();
        self.match_cache
            .retain(|_, (_, ts)| now.duration_since(*ts) < self.cache_ttl);
    }

    #[allow(dead_code)]
    pub fn index_size(&self) -> usize {
        self.embedding_index.len()
    }

    #[allow(dead_code)]
    pub fn cache_size(&self) -> usize {
        self.match_cache.len()
    }

    // ── Public entry point ──────────────────────────────────────────────

    /// Two-stage matching: embedding retrieval → LLM rerank, with caching.
    pub async fn match_market(
        &mut self,
        poly_title: &str,
        poly_outcome: &str,
        candidates: &[crate::platforms::kalshi::MarketInfo],
    ) -> Option<MatchResult> {
        if candidates.is_empty() {
            return None;
        }

        // ── Cache hit? ─────────────────────────────────────────────────
        let key = cache_key(poly_title, poly_outcome);
        if let Some((cached, ts)) = self.match_cache.get(&key) {
            if ts.elapsed() < self.cache_ttl {
                println!("⚡ Cache hit for: \"{}\"", poly_title);
                return Some(cached.clone());
            }
        }

        // ── Stage 1: retrieve candidates ───────────────────────────────
        let shortlist = self.retrieve_candidates(poly_title, candidates).await;
        if shortlist.is_empty() {
            println!("⚠️ No candidates found for: {}", poly_title);
            return None;
        }

        println!(
            "🔍 Stage 1 → {} candidates for \"{}\"",
            shortlist.len(),
            poly_title
        );

        // ── Stage 2: LLM rerank ───────────────────────────────────────
        let result = self.llm_rerank(poly_title, poly_outcome, &shortlist).await;

        if let Some(ref r) = result {
            self.match_cache.insert(key, (r.clone(), Instant::now()));
        }

        result
    }

    // ── Embedding helpers ───────────────────────────────────────────────

    async fn embed_batch(&self, texts: &[&str]) -> Option<Vec<Vec<f32>>> {
        let body = json!({
            "model": self.embed_model,
            "input": texts
        });

        let resp = self
            .client
            .post(&self.embed_url)
            .json(&body)
            .timeout(Duration::from_secs(60))
            .send()
            .await
            .ok()?;

        let data: OllamaEmbedResponse = resp.json().await.ok()?;
        if data.embeddings.len() == texts.len() {
            Some(data.embeddings)
        } else {
            None
        }
    }

    async fn embed_single(&self, text: &str) -> Option<Vec<f32>> {
        // nomic-embed-text uses "search_query: " prefix for queries
        let prefixed = format!("search_query: {}", text);
        self.embed_batch(&[&prefixed])
            .await
            .and_then(|mut v| v.pop())
    }

    // ── Stage 1: hybrid candidate retrieval ──────────────────────────────

    async fn retrieve_candidates<'a>(
        &self,
        poly_title: &str,
        candidates: &'a [crate::platforms::kalshi::MarketInfo],
    ) -> Vec<&'a crate::platforms::kalshi::MarketInfo> {
        let keywords = expand_keywords(poly_title);
        let max_kw = keywords.len().max(1) as f32;

        let query_emb = if !self.embedding_index.is_empty() {
            self.embed_single(poly_title).await
        } else {
            None
        };

        // Score each candidate with a hybrid of keyword overlap + cosine similarity.
        // Keyword overlap captures entity names (team, ticker, date); cosine captures
        // semantic meaning ("BTC" vs "Bitcoin").  When embeddings are unavailable the
        // keyword score alone drives ranking.
        let mut scored: Vec<(&crate::platforms::kalshi::MarketInfo, f32)> = candidates
            .iter()
            .map(|m| {
                let title_lower = m.title.to_lowercase();
                let kw_hits = keywords
                    .iter()
                    .filter(|kw| title_lower.contains(kw.as_str()))
                    .count() as f32;
                let kw_score = kw_hits / max_kw; // 0..1

                let emb_score = query_emb
                    .as_ref()
                    .and_then(|qe| {
                        self.embedding_index
                            .get(&m.ticker)
                            .map(|de| cosine_similarity(qe, de))
                    })
                    .unwrap_or(0.0);

                // Weighted combination: keywords dominate when entity names exist;
                // embeddings help when keywords miss (abbreviation gaps, synonyms).
                let combined = 0.6 * kw_score + 0.4 * emb_score;
                (m, combined)
            })
            .filter(|(_, score)| *score > 0.01)
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
            .into_iter()
            .take(DEFAULT_TOP_K)
            .map(|(m, _)| m)
            .collect()
    }

    // ── Stage 2: LLM rerank ────────────────────────────────────────────

    async fn llm_rerank(
        &self,
        poly_title: &str,
        poly_outcome: &str,
        candidates: &[&crate::platforms::kalshi::MarketInfo],
    ) -> Option<MatchResult> {
        let valid_tickers: std::collections::HashSet<&str> =
            candidates.iter().map(|m| m.ticker.as_str()).collect();

        let mut candidate_str = String::new();
        for (i, m) in candidates.iter().enumerate() {
            candidate_str.push_str(&format!(
                "{}. Ticker: {} | Title: {}\n",
                i + 1,
                m.ticker,
                m.title
            ));
        }
        println!(
            "   Candidate tickers: {:?}",
            candidates.iter().map(|m| &m.ticker).collect::<Vec<_>>()
        );

        let expanded = expand_keywords(poly_title);
        let alias_hint = if expanded.len() > 2 {
            format!(
                "\nNOTE: \"{}\" is also known as / related to: {}",
                poly_title,
                expanded.join(", ")
            )
        } else {
            String::new()
        };

        let prompt = format!(
            r#"Task: Determine if ANY of the candidate Kalshi markets below resolves on the EXACT SAME real-world outcome as the Polymarket event.
{alias_hint}
IMPORTANT SPORTS CONTEXT: In prediction markets, the same game is often listed with different naming conventions:
- Polymarket uses team NICKNAMES: "Celtics vs. Warriors", "Nuggets vs. Clippers"
- Kalshi uses CITY names: "Boston at Golden State Winner?", "Denver at Los Angeles C Winner?"
- These are the SAME events if the teams match!

RULES:
1. The Polymarket event and the Kalshi market MUST be about the SAME specific real-world event.
2. For sports: team nickname = city name (e.g. Celtics = Boston, Warriors = Golden State, Nuggets = Denver, Clippers = Los Angeles C).
3. If NO candidate is a genuine match, return "match": false.
4. The "ticker" field MUST be copied EXACTLY from the candidate list. Do not invent tickers.
5. For binary-outcome markets with two tickers (e.g. -BOS and -GSW), pick the ticker whose side matches the Polymarket outcome.

Polymarket Alert: "{poly_title}" (Outcome: {poly_outcome})

Candidate Kalshi Markets (ranked by initial relevance):
{candidate_str}
Output ONLY valid JSON:
{{
    "match": true or false,
    "ticker": "EXACT ticker from candidate list above (only if match is true)",
    "side": "yes or no (only if match is true)",
    "confidence": 0.0 to 1.0,
    "reasoning": "Explain why events are or are not equivalent"
}}"#
        );

        let body = json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
            "format": "json"
        });

        match self
            .client
            .post(&self.generate_url)
            .json(&body)
            .timeout(Duration::from_secs(60))
            .send()
            .await
        {
            Ok(resp) => {
                if let Ok(ollama_resp) = resp.json::<OllamaResponse>().await {
                    let json_str = strip_json_fences(&ollama_resp.response);
                    println!("🤖 LLM rerank: {}", json_str);
                    if let Ok(result) = serde_json::from_str::<MatchResult>(json_str) {
                        let conf = result.confidence.unwrap_or(0.0);
                        let tier = confidence_tier(conf);
                        println!("✅ Match: {} | Tier: {:?} ({:.2})", result.ticker, tier, conf);

                        if !result.r#match || result.ticker.is_empty() {
                            println!("⚠️ LLM returned no match or empty ticker");
                            return None;
                        }
                        if !valid_tickers.contains(result.ticker.as_str()) {
                            println!(
                                "🚫 REJECTED: LLM returned ticker '{}' not in candidate list — hallucination blocked",
                                result.ticker
                            );
                            return None;
                        }
                        if tier == ConfidenceTier::None {
                            println!("⚠️ Low confidence ({:.2}), skipping", conf);
                            return None;
                        }
                        return Some(result);
                    } else {
                        println!("❌ Failed to parse LLM response as MatchResult");
                    }
                }
            }
            Err(e) => eprintln!("[ERROR] Ollama request failed: {}", e),
        }
        None
    }
}

// ── Keyword filter (standalone for fallback and tests) ──────────────────

#[allow(dead_code)]
pub fn keyword_filter<'a>(
    poly_title: &str,
    candidates: &'a [crate::platforms::kalshi::MarketInfo],
) -> Vec<&'a crate::platforms::kalshi::MarketInfo> {
    let keywords = expand_keywords(poly_title);
    if keywords.is_empty() {
        return candidates.iter().take(DEFAULT_TOP_K).collect();
    }

    let mut scored: Vec<(&crate::platforms::kalshi::MarketInfo, usize)> = candidates
        .iter()
        .map(|c| {
            let title_lower = c.title.to_lowercase();
            let count = keywords
                .iter()
                .filter(|kw| title_lower.contains(kw.as_str()))
                .count();
            (c, count)
        })
        .filter(|(_, count)| *count > 0)
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored
        .into_iter()
        .take(DEFAULT_TOP_K + 10)
        .map(|(m, _)| m)
        .collect()
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── cosine_similarity ───────────────────────────────────────────────

    #[test]
    fn cosine_identical_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        assert!((cosine_similarity(&a, &a) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn cosine_orthogonal_vectors() {
        assert!(cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-5);
    }

    #[test]
    fn cosine_opposite_vectors() {
        assert!((cosine_similarity(&[1.0, 0.0], &[-1.0, 0.0]) + 1.0).abs() < 1e-5);
    }

    #[test]
    fn cosine_empty_returns_zero() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
    }

    #[test]
    fn cosine_mismatched_lengths_returns_zero() {
        assert_eq!(cosine_similarity(&[1.0], &[1.0, 2.0]), 0.0);
    }

    #[test]
    fn cosine_zero_vector_returns_zero() {
        assert_eq!(cosine_similarity(&[0.0, 0.0], &[1.0, 2.0]), 0.0);
    }

    #[test]
    fn cosine_similar_vectors_high() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.1, 2.1, 3.1];
        let sim = cosine_similarity(&a, &b);
        assert!(sim > 0.99, "Expected high similarity, got {}", sim);
    }

    // ── confidence_tier ─────────────────────────────────────────────────

    #[test]
    fn tier_exact() {
        assert_eq!(confidence_tier(0.95), ConfidenceTier::Exact);
        assert_eq!(confidence_tier(1.0), ConfidenceTier::Exact);
    }

    #[test]
    fn tier_related() {
        assert_eq!(confidence_tier(0.80), ConfidenceTier::Related);
        assert_eq!(confidence_tier(0.90), ConfidenceTier::Related);
        assert_eq!(confidence_tier(0.949), ConfidenceTier::Related);
    }

    #[test]
    fn tier_none() {
        assert_eq!(confidence_tier(0.79), ConfidenceTier::None);
        assert_eq!(confidence_tier(0.0), ConfidenceTier::None);
    }

    // ── expand_keywords ─────────────────────────────────────────────────

    #[test]
    fn expand_btc_to_bitcoin() {
        let words = expand_keywords("Will BTC reach $100k?");
        assert!(words.contains(&"btc".to_string()));
        assert!(words.contains(&"bitcoin".to_string()));
        assert!(words.contains(&"reach".to_string()));
        assert!(words.contains(&"100k".to_string()));
        assert!(!words.contains(&"will".to_string()));
    }

    #[test]
    fn expand_sports_abbreviations() {
        let words = expand_keywords("NBA Finals Game 5");
        assert!(words.contains(&"nba".to_string()));
        assert!(words.contains(&"basketball".to_string()));
        assert!(words.contains(&"finals".to_string()));
    }

    #[test]
    fn expand_politics() {
        let words = expand_keywords("Fed interest rate decision");
        assert!(words.contains(&"fed".to_string()));
        assert!(words.contains(&"federal".to_string()));
        assert!(words.contains(&"reserve".to_string()));
        assert!(words.contains(&"interest".to_string()));
    }

    #[test]
    fn expand_over_under() {
        let words = expand_keywords("O/U 2.5 goals");
        assert!(words.contains(&"o/u".to_string()));
        assert!(words.contains(&"over".to_string()));
        assert!(words.contains(&"under".to_string()));
    }

    #[test]
    fn expand_removes_stop_words() {
        let words = expand_keywords("Will the price have outcome");
        assert!(!words.contains(&"will".to_string()));
        assert!(!words.contains(&"the".to_string()));
        assert!(!words.contains(&"price".to_string()));
        assert!(!words.contains(&"outcome".to_string()));
    }

    #[test]
    fn expand_strips_punctuation() {
        let words = expand_keywords("Hello, world! $100?");
        assert!(words.contains(&"hello".to_string()));
        assert!(words.contains(&"world".to_string()));
        assert!(words.contains(&"100".to_string()));
    }

    // ── strip_json_fences ───────────────────────────────────────────────

    #[test]
    fn strip_json_tag_fences() {
        assert_eq!(
            strip_json_fences("```json\n{\"match\": true}\n```"),
            "{\"match\": true}"
        );
    }

    #[test]
    fn strip_plain_fences() {
        assert_eq!(
            strip_json_fences("```\n{\"match\": true}\n```"),
            "{\"match\": true}"
        );
    }

    #[test]
    fn strip_no_fences() {
        assert_eq!(strip_json_fences("{\"match\": true}"), "{\"match\": true}");
    }

    #[test]
    fn strip_with_extra_whitespace() {
        let raw = "  ```json\n  {\"a\": 1}  \n```  ";
        let result = strip_json_fences(raw);
        assert!(result.contains("{\"a\": 1}"));
    }

    // ── cache_key ───────────────────────────────────────────────────────

    #[test]
    fn cache_key_deterministic() {
        assert_eq!(
            cache_key("test title", "YES"),
            cache_key("test title", "YES")
        );
    }

    #[test]
    fn cache_key_differs_on_title() {
        assert_ne!(cache_key("A", "YES"), cache_key("B", "YES"));
    }

    #[test]
    fn cache_key_differs_on_outcome() {
        assert_ne!(cache_key("A", "YES"), cache_key("A", "NO"));
    }

    // ── keyword_filter ──────────────────────────────────────────────────

    fn make_market(ticker: &str, title: &str) -> crate::platforms::kalshi::MarketInfo {
        crate::platforms::kalshi::MarketInfo {
            ticker: ticker.to_string(),
            title: title.to_string(),
            category: None,
            tags: vec![],
        }
    }

    #[test]
    fn keyword_filter_ranks_by_overlap() {
        let markets = vec![
            make_market("A", "Bitcoin price over 100k"),
            make_market("B", "Ethereum staking rewards"),
            make_market("C", "Will Bitcoin BTC reach 100k by March"),
        ];
        let results = keyword_filter("BTC reach 100k", &markets);
        assert!(
            !results.is_empty(),
            "Should find at least one matching market"
        );
        // "C" has BTC + 100k + reach → more overlap than "A" (bitcoin + 100k)
        assert_eq!(results[0].ticker, "C");
    }

    #[test]
    fn keyword_filter_returns_empty_on_no_match() {
        let markets = vec![
            make_market("A", "Weather forecast tomorrow"),
            make_market("B", "Celebrity gossip news"),
        ];
        let results = keyword_filter("Bitcoin ETF approval", &markets);
        assert!(results.is_empty());
    }

    #[test]
    fn keyword_filter_expands_abbreviations() {
        let markets = vec![
            make_market("A", "Bitcoin to reach $100,000 by end of year"),
            make_market("B", "Ethereum price drop"),
        ];
        // "BTC" expands to include "bitcoin", so "A" should match
        let results = keyword_filter("BTC $100k", &markets);
        assert!(!results.is_empty());
        assert_eq!(results[0].ticker, "A");
    }

    #[test]
    fn keyword_filter_limits_results() {
        let markets: Vec<crate::platforms::kalshi::MarketInfo> = (0..100)
            .map(|i| make_market(&format!("T{}", i), &format!("Bitcoin event {}", i)))
            .collect();
        let results = keyword_filter("Bitcoin event", &markets);
        assert!(
            results.len() <= DEFAULT_TOP_K + 10,
            "Should limit results, got {}",
            results.len()
        );
    }

    // ── MatchResult JSON parsing ────────────────────────────────────────

    #[test]
    fn parse_valid_match_result() {
        let json = r#"{"match":true,"ticker":"KXFB-WIN","side":"yes","confidence":0.95,"reasoning":"same event"}"#;
        let result: MatchResult = serde_json::from_str(json).unwrap();
        assert!(result.r#match);
        assert_eq!(result.ticker, "KXFB-WIN");
        assert_eq!(result.side, "yes");
        assert!((result.confidence.unwrap() - 0.95).abs() < 1e-5);
    }

    #[test]
    fn parse_match_result_without_optional_fields() {
        let json = r#"{"match":false,"ticker":"","side":"","confidence":null}"#;
        let result: MatchResult = serde_json::from_str(json).unwrap();
        assert!(!result.r#match);
        assert!(result.confidence.is_none());
        assert!(result.reasoning.is_none());
    }

    // ── Integration: end-to-end candidate scoring ───────────────────────

    #[test]
    fn cosine_ranks_similar_titles_higher() {
        let query = vec![0.9, 0.1, 0.0];
        let market_sports = vec![0.85, 0.15, 0.0];
        let market_crypto = vec![0.0, 0.1, 0.9];

        let sim_sports = cosine_similarity(&query, &market_sports);
        let sim_crypto = cosine_similarity(&query, &market_crypto);

        assert!(
            sim_sports > sim_crypto,
            "Sports market ({:.3}) should score higher than crypto ({:.3})",
            sim_sports,
            sim_crypto
        );
        assert!(sim_sports > 0.9);
        assert!(sim_crypto < 0.3);
    }

    // ── Hybrid scoring ──────────────────────────────────────────────────

    /// Verify that keyword overlap + cosine similarity combined puts the
    /// correct match at the top even when cosine alone is ambiguous.
    #[test]
    fn hybrid_score_keyword_plus_cosine() {
        // query = "Will Lille OSC win?" → keywords: ["lille", "osc", "win", ...]
        // Candidate A: "Lille OSC to Win Feb 14" — high keyword, medium cosine
        // Candidate B: "Fed interest rate March" — zero keyword overlap, higher cosine
        let keywords = expand_keywords("Will Lille OSC win on 2026-02-14?");
        let max_kw = keywords.len().max(1) as f32;

        let kw_hits_a = keywords
            .iter()
            .filter(|kw| "lille osc to win feb 14".contains(kw.as_str()))
            .count() as f32;
        let kw_hits_b = keywords
            .iter()
            .filter(|kw| "fed interest rate march".contains(kw.as_str()))
            .count() as f32;

        // Simulate cosine scores: unrelated candidate has higher cosine
        let cosine_a = 0.55_f32;
        let cosine_b = 0.77_f32;

        let score_a = 0.6 * (kw_hits_a / max_kw) + 0.4 * cosine_a;
        let score_b = 0.6 * (kw_hits_b / max_kw) + 0.4 * cosine_b;

        assert!(
            kw_hits_a > 0.0,
            "Lille candidate should have keyword hits"
        );
        assert_eq!(
            kw_hits_b, 0.0,
            "Fed candidate should have zero keyword hits"
        );
        assert!(
            score_a > score_b,
            "Hybrid: Lille ({:.3}) should beat Fed ({:.3}) despite lower cosine",
            score_a,
            score_b
        );
    }

    /// When both candidates share keywords, the one with more overlap wins.
    #[test]
    fn hybrid_score_prefers_more_keyword_overlap() {
        let keywords = expand_keywords("BTC reach 100k");
        let max_kw = keywords.len().max(1) as f32;

        let kw_a = keywords
            .iter()
            .filter(|kw| "bitcoin to reach 100000".contains(kw.as_str()))
            .count() as f32;
        let kw_b = keywords
            .iter()
            .filter(|kw| "bitcoin etf approval".contains(kw.as_str()))
            .count() as f32;

        // Equal cosine (both crypto-related)
        let cosine = 0.6_f32;
        let score_a = 0.6 * (kw_a / max_kw) + 0.4 * cosine;
        let score_b = 0.6 * (kw_b / max_kw) + 0.4 * cosine;

        assert!(
            kw_a > kw_b,
            "BTC-100k match should have more keyword overlap"
        );
        assert!(
            score_a > score_b,
            "Higher keyword overlap should win: {:.3} > {:.3}",
            score_a,
            score_b
        );
    }

    // ════════════════════════════════════════════════════════════════════
    //  END-TO-END INTEGRATION TESTS (require Ollama running locally)
    // ════════════════════════════════════════════════════════════════════

    fn ollama_available() -> bool {
        std::net::TcpStream::connect("127.0.0.1:11434").is_ok()
    }

    fn test_markets() -> Vec<crate::platforms::kalshi::MarketInfo> {
        vec![
            make_market("KXFB-26FEB14-LILLEWIN", "Will Lille OSC win on February 14?"),
            make_market("KXFB-26FEB14-LILLEOU25", "Lille vs Monaco: Over/Under 2.5 goals"),
            make_market("KXFB-26FEB14-NEWCWIN", "Will Newcastle United FC win on February 14?"),
            make_market("KXCRYPTO-BTC100K-MAR", "Bitcoin to reach $100,000 by March 2026"),
            make_market("KXCRYPTO-ETH5K", "Ethereum above $5,000 by end of Q1 2026"),
            make_market("KXPOL-FEDRATE-MAR26", "Fed to cut interest rates at March 2026 meeting"),
            make_market("KXPOL-CEASEFIRE-JUN", "Russia Ukraine ceasefire by June 30 2026"),
            make_market("KXSPORT-NBAMVP26", "NBA MVP 2026 Winner"),
            make_market("KXSPORT-KANSAS-IOWA", "Kansas Jayhawks vs Iowa State Cyclones"),
            make_market("KXPOL-USIRAN-MAR", "US strikes Iran by March 31 2026"),
            make_market("KXESPORT-DOTA-YANDEX", "Dota 2: Team Yandex vs Natus Vincere Game 1 Winner"),
            make_market("KXFB-26FEB14-LAZIOWIN", "Will SS Lazio win on February 14?"),
            make_market("KXFB-26FEB14-LAZIOOU25", "SS Lazio vs Atalanta BC: Over/Under 2.5"),
            make_market("KXFB-26FEB14-INTERWIN", "Will FC Internazionale Milano win on February 14?"),
            make_market("KXCRYPTO-SATOSHI26", "Will Satoshi move any Bitcoin in 2026?"),
        ]
    }

    // ── E2E: Embedding index build ──────────────────────────────────────

    #[tokio::test]
    async fn e2e_build_embedding_index() {
        if !ollama_available() {
            eprintln!("SKIP: Ollama not running");
            return;
        }
        let mut matcher = MarketMatcher::new(
            "llama3".into(),
            "nomic-embed-text".into(),
            Some("http://localhost:11434"),
        );
        let markets = test_markets();

        matcher.build_index(&markets).await;

        assert_eq!(
            matcher.index_size(),
            markets.len(),
            "All {} markets should be indexed",
            markets.len()
        );
    }

    // ── E2E: Hybrid retrieval returns correct top candidates ────────────

    #[tokio::test]
    async fn e2e_retrieve_lille_match() {
        if !ollama_available() {
            eprintln!("SKIP: Ollama not running");
            return;
        }
        let mut matcher = MarketMatcher::new(
            "llama3".into(),
            "nomic-embed-text".into(),
            Some("http://localhost:11434"),
        );
        let markets = test_markets();
        matcher.build_index(&markets).await;

        let candidates = matcher
            .retrieve_candidates("Will Lille OSC win on 2026-02-14?", &markets)
            .await;

        assert!(
            !candidates.is_empty(),
            "Should find candidates for Lille query"
        );

        let tickers: Vec<&str> = candidates.iter().map(|m| m.ticker.as_str()).collect();
        assert!(
            tickers.contains(&"KXFB-26FEB14-LILLEWIN"),
            "Lille win market should be in top candidates, got: {:?}",
            tickers
        );
    }

    #[tokio::test]
    async fn e2e_retrieve_btc_match() {
        if !ollama_available() {
            eprintln!("SKIP: Ollama not running");
            return;
        }
        let mut matcher = MarketMatcher::new(
            "llama3".into(),
            "nomic-embed-text".into(),
            Some("http://localhost:11434"),
        );
        let markets = test_markets();
        matcher.build_index(&markets).await;

        let candidates = matcher
            .retrieve_candidates("Will BTC reach $100k?", &markets)
            .await;

        let tickers: Vec<&str> = candidates.iter().map(|m| m.ticker.as_str()).collect();
        assert!(
            tickers.contains(&"KXCRYPTO-BTC100K-MAR"),
            "BTC 100k market should be in candidates, got: {:?}",
            tickers
        );
    }

    #[tokio::test]
    async fn e2e_retrieve_fed_rates_match() {
        if !ollama_available() {
            eprintln!("SKIP: Ollama not running");
            return;
        }
        let mut matcher = MarketMatcher::new(
            "llama3".into(),
            "nomic-embed-text".into(),
            Some("http://localhost:11434"),
        );
        let markets = test_markets();
        matcher.build_index(&markets).await;

        let candidates = matcher
            .retrieve_candidates(
                "Will the Fed decrease interest rates by 25 bps after the March 2026 meeting?",
                &markets,
            )
            .await;

        let tickers: Vec<&str> = candidates.iter().map(|m| m.ticker.as_str()).collect();
        assert!(
            tickers.contains(&"KXPOL-FEDRATE-MAR26"),
            "Fed rate market should be in candidates, got: {:?}",
            tickers
        );
    }

    // ── E2E: Full match_market (embedding + LLM rerank) ────────────────
    // These tests require Ollama with llama3 + nomic-embed-text. LLM output is
    // non-deterministic — run with `cargo test -- --ignored` to execute them.

    #[tokio::test]
    #[ignore = "requires Ollama; LLM output non-deterministic"]
    async fn e2e_match_market_lille() {
        if !ollama_available() {
            eprintln!("SKIP: Ollama not running");
            return;
        }
        let mut matcher = MarketMatcher::new(
            "llama3".into(),
            "nomic-embed-text".into(),
            Some("http://localhost:11434"),
        );
        let markets = test_markets();
        matcher.build_index(&markets).await;

        let result = matcher
            .match_market("Will Lille OSC win on 2026-02-14?", "Yes", &markets)
            .await;

        assert!(result.is_some(), "Should find a match for Lille OSC");
        let m = result.unwrap();
        assert!(m.r#match, "match should be true");
        assert_eq!(m.ticker, "KXFB-26FEB14-LILLEWIN");
        let conf = m.confidence.unwrap_or(0.0);
        assert!(
            conf >= 0.80,
            "Confidence should be >= 0.80, got {:.2}",
            conf
        );
        println!("  Lille match: ticker={} side={} conf={:.2} reasoning={:?}", m.ticker, m.side, conf, m.reasoning);
    }

    #[tokio::test]
    #[ignore = "requires Ollama; LLM output non-deterministic"]
    async fn e2e_match_market_btc() {
        if !ollama_available() {
            eprintln!("SKIP: Ollama not running");
            return;
        }
        let mut matcher = MarketMatcher::new(
            "llama3".into(),
            "nomic-embed-text".into(),
            Some("http://localhost:11434"),
        );
        let markets = test_markets();
        matcher.build_index(&markets).await;

        let result = matcher
            .match_market("Will BTC reach $100k?", "Yes", &markets)
            .await;

        assert!(result.is_some(), "Should find a match for BTC 100k");
        let m = result.unwrap();
        assert_eq!(m.ticker, "KXCRYPTO-BTC100K-MAR");
        println!("  BTC match: ticker={} side={} conf={:.2}", m.ticker, m.side, m.confidence.unwrap_or(0.0));
    }

    #[tokio::test]
    #[ignore = "requires Ollama; LLM output non-deterministic"]
    async fn e2e_match_market_ceasefire() {
        if !ollama_available() {
            eprintln!("SKIP: Ollama not running");
            return;
        }
        let mut matcher = MarketMatcher::new(
            "llama3".into(),
            "nomic-embed-text".into(),
            Some("http://localhost:11434"),
        );
        let markets = test_markets();
        matcher.build_index(&markets).await;

        let result = matcher
            .match_market(
                "Russia x Ukraine ceasefire by June 30, 2026?",
                "Yes",
                &markets,
            )
            .await;

        assert!(result.is_some(), "Should find a match for ceasefire");
        let m = result.unwrap();
        assert_eq!(m.ticker, "KXPOL-CEASEFIRE-JUN");
        println!("  Ceasefire match: ticker={} side={} conf={:.2}", m.ticker, m.side, m.confidence.unwrap_or(0.0));
    }

    // ── E2E: Cache hit on second call ──────────────────────────────────

    #[tokio::test]
    #[ignore = "requires Ollama; LLM output non-deterministic"]
    async fn e2e_cache_hit_second_call() {
        if !ollama_available() {
            eprintln!("SKIP: Ollama not running");
            return;
        }
        let mut matcher = MarketMatcher::new(
            "llama3".into(),
            "nomic-embed-text".into(),
            Some("http://localhost:11434"),
        );
        let markets = test_markets();
        matcher.build_index(&markets).await;

        // First call — cold
        let r1 = matcher
            .match_market("Will Lille OSC win on 2026-02-14?", "Yes", &markets)
            .await;
        assert!(r1.is_some());
        assert_eq!(matcher.cache_size(), 1);

        // Second call — should be instant cache hit
        let start = Instant::now();
        let r2 = matcher
            .match_market("Will Lille OSC win on 2026-02-14?", "Yes", &markets)
            .await;
        let elapsed = start.elapsed();

        assert!(r2.is_some());
        assert_eq!(
            r1.unwrap().ticker,
            r2.unwrap().ticker,
            "Cache should return same result"
        );
        assert!(
            elapsed.as_millis() < 50,
            "Cache hit should be < 50ms, was {}ms",
            elapsed.as_millis()
        );
    }

    // ── E2E: No match for unrelated query ───────────────────────────────

    #[tokio::test]
    #[ignore = "requires Ollama; LLM output non-deterministic"]
    async fn e2e_no_match_unrelated() {
        if !ollama_available() {
            eprintln!("SKIP: Ollama not running");
            return;
        }
        let mut matcher = MarketMatcher::new(
            "llama3".into(),
            "nomic-embed-text".into(),
            Some("http://localhost:11434"),
        );
        let markets = test_markets();
        matcher.build_index(&markets).await;

        let result = matcher
            .match_market(
                "Will Taylor Swift release a new album in 2026?",
                "Yes",
                &markets,
            )
            .await;

        // None of the test markets match Taylor Swift
        assert!(
            result.is_none(),
            "Should not match Taylor Swift to any market, got: {:?}",
            result
        );
    }
}
