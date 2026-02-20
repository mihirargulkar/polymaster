use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone)]
pub struct MarketMatcher {
    client: reqwest::Client,
    model: String,
    base_url: String,
}

#[derive(Deserialize, Debug)]
struct OllamaResponse {
    response: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct MatchResult {
    pub r#match: bool,
    pub ticker: String,
    pub side: String,
    pub confidence: Option<f64>,
    pub reasoning: Option<String>,
}

impl MarketMatcher {
    pub fn new(model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            model,
            base_url: "http://localhost:11434/api/generate".to_string(),
        }
    }

    pub async fn match_market(
        &self,
        poly_title: &str,
        poly_outcome: &str,
        candidates: &[crate::platforms::kalshi::MarketInfo],
    ) -> Option<MatchResult> {
        println!("🔍 Matching: \"{}\" ({}) with {} candidates", poly_title, poly_outcome, candidates.len());
        if candidates.is_empty() {
            return None;
        }

        // 1. Pre-filter candidates by simple keyword matching to reduce LLM tokens
        let poly_words: Vec<String> = poly_title.to_lowercase()
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .map(|w| w.to_string())
            .collect();

        let filtered_candidates: Vec<&crate::platforms::kalshi::MarketInfo> = if poly_words.is_empty() {
            candidates.iter().take(20).collect() // Fallback
        } else {
            let mut matches: Vec<(&crate::platforms::kalshi::MarketInfo, usize)> = candidates.iter()
                .map(|c| {
                    let title_lower = c.title.to_lowercase();
                    let count = poly_words.iter().filter(|w| title_lower.contains(w.as_str())).count();
                    (c, count)
                })
                .filter(|(_, count)| *count > 0)
                .collect();
            
            matches.sort_by(|a, b| b.1.cmp(&a.1));
            matches.into_iter().take(25).map(|(c, _)| c).collect()
        };

        if filtered_candidates.is_empty() {
            println!("⚠️ No candidates passed keyword filter for: {}", poly_title);
            return None;
        }

        println!("🔍 Matching: \"{}\" ({}) with {} filtered candidates", poly_title, poly_outcome, filtered_candidates.len());

        let mut candidate_str = String::new();
        for (i, m) in filtered_candidates.iter().enumerate() {
            candidate_str.push_str(&format!("{}. Ticker: {} | Title: {}\n", i + 1, m.ticker, m.title));
        }

        let prompt = format!(
            r#"Task: Match the Polymarket event to the equivalent Kalshi market.

Polymarket Alert: "{}" (Outcome: {})

Candidate Kalshi Markets:
{}

Output JSON ONLY:
{{
    "match": true/false,
    "ticker": "Ticker symbol of the match",
    "side": "yes/no (which side of the Kalshi market maps to the Polymarket outcome)",
    "confidence": 0.0 to 1.0,
    "reasoning": "Short explanation of why this matches"
}}"#,
            poly_title, poly_outcome, candidate_str
        );

        let body = json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
            "format": "json"
        });

        match self.client.post(&self.base_url).json(&body).send().await {
            Ok(resp) => {
                if let Ok(ollama_resp) = resp.json::<OllamaResponse>().await {
                    println!("🤖 LLM Response: {}", ollama_resp.response);
                    if let Ok(result) = serde_json::from_str::<MatchResult>(&ollama_resp.response) {
                        println!("✅ Parsed Match: {:?}", result);
                        // Default confidence to 0.0 if missing
                        let conf = result.confidence.unwrap_or(0.0);
                        if result.r#match && conf > 0.8 {
                            return Some(result);
                        } else {
                            println!("⚠️ Low confidence ({}) or no match", conf);
                        }
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
