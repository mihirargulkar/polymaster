/**
 * Perplexity API client for deep web research
 * Used for gathering comprehensive market intelligence
 */

import type { AlertScore, ResearchSignal } from "../util/types.js";

export interface PerplexityResponse {
  query: string;
  answer: string;
  citations: string[];
  error?: string;
}

export interface PerplexitySearchResult {
  queries: string[];
  results: PerplexityResponse[];
  summary?: string;
}

/**
 * Query Perplexity API for market research
 */
export async function queryPerplexity(
  query: string,
  apiKey: string,
  model: string = "sonar"
): Promise<PerplexityResponse> {
  try {
    const response = await fetch("https://api.perplexity.ai/chat/completions", {
      method: "POST",
      headers: {
        "Authorization": `Bearer ${apiKey}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        model,
        messages: [
          {
            role: "system",
            content: "You are a research assistant focused on prediction markets, financial analysis, and current events. Provide concise, factual answers with relevant data points. Focus on information that would help predict market outcomes."
          },
          {
            role: "user",
            content: query
          }
        ],
        max_tokens: 1024,
        temperature: 0.2,
        return_citations: true,
      }),
    });

    if (!response.ok) {
      const errorText = await response.text();
      return {
        query,
        answer: "",
        citations: [],
        error: `Perplexity API error (${response.status}): ${errorText}`,
      };
    }

    const data = await response.json();
    const answer = data.choices?.[0]?.message?.content || "";
    const citations = data.citations || [];

    return {
      query,
      answer,
      citations,
    };
  } catch (err) {
    return {
      query,
      answer: "",
      citations: [],
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

/**
 * Generate generic research queries for a market (legacy, no alert context)
 */
export function generateResearchQueries(
  marketTitle: string,
  category?: string
): string[] {
  const baseQueries = [
    `Latest news and developments: ${marketTitle}`,
    `Expert analysis and predictions: ${marketTitle}`,
    `Historical data and trends: ${marketTitle}`,
    `Risk factors and uncertainties: ${marketTitle}`,
    `Recent events affecting: ${marketTitle}`,
  ];

  // Add category-specific queries
  if (category === "crypto") {
    baseQueries.push(
      `${marketTitle} - technical analysis and price targets`,
      `${marketTitle} - whale activity and institutional interest`
    );
  } else if (category === "sports") {
    baseQueries.push(
      `${marketTitle} - injury reports and team news`,
      `${marketTitle} - betting odds movement and sharp money`
    );
  } else if (category === "weather") {
    baseQueries.push(
      `${marketTitle} - forecast models and confidence levels`,
      `${marketTitle} - historical weather patterns`
    );
  } else if (category === "politics") {
    baseQueries.push(
      `${marketTitle} - polling data and trends`,
      `${marketTitle} - key demographics and swing factors`
    );
  }

  return baseQueries.slice(0, 5); // Return top 5 queries
}

/**
 * Generate context-aware research queries based on alert scoring.
 * Uses whale profile, order book, and position data to ask sharper questions.
 * Returns 3 targeted queries instead of 5 generic ones.
 */
export function generateContextQueries(
  marketTitle: string,
  score: AlertScore,
  alertContext: {
    action: string;
    outcome?: string | null;
    value: number;
    platform: string;
    winRate?: number | null;
    rank?: number | null;
    bidImbalance?: number;
    isHeavyActor?: boolean;
    isContrarian?: boolean;
    tags?: string[];
  }
): string[] {
  const queries: string[] = [];

  // Query 1: Always include a grounding query with latest news
  queries.push(`Latest news and key developments on "${marketTitle}" in the last 48 hours`);

  // Query 2: Whale-context query based on scoring factors
  if (score.tier === "high") {
    if (alertContext.rank && alertContext.rank <= 50 && alertContext.winRate) {
      queries.push(
        `Why would a top-${alertContext.rank <= 10 ? "10" : "50"} prediction market trader with ${(alertContext.winRate * 100).toFixed(0)}% win rate bet $${formatCompact(alertContext.value)} ${alertContext.action} on "${marketTitle}"? What information might they have?`
      );
    } else if (alertContext.isHeavyActor) {
      queries.push(
        `What would drive repeated large bets on "${marketTitle}"? This trader has made 5+ transactions in 24 hours totaling $${formatCompact(alertContext.value)}+.`
      );
    } else {
      queries.push(
        `What factors support a $${formatCompact(alertContext.value)} ${alertContext.action} position on "${marketTitle}"? Analyze the bull and bear case.`
      );
    }
  } else if (score.tier === "medium") {
    queries.push(
      `Analysis of "${marketTitle}": what are the key factors that could move this market in either direction?`
    );
  } else {
    queries.push(
      `Quick overview of "${marketTitle}": current state and near-term catalysts`
    );
  }

  // Query 3: Contrarian or directional query
  if (alertContext.isContrarian) {
    queries.push(
      `What catalysts could flip the consensus on "${marketTitle}"? A large trader is betting against the current market odds.`
    );
  } else if (alertContext.bidImbalance !== undefined && alertContext.bidImbalance >= 0.65) {
    queries.push(
      `"${marketTitle}" shows strong one-sided order flow. Is there insider knowledge or a fundamental catalyst driving this conviction?`
    );
  } else {
    // Category-specific third query
    const category = alertContext.tags?.[0]?.toLowerCase();
    if (category === "crypto" || alertContext.tags?.some(t => ["bitcoin", "btc", "ethereum", "eth"].includes(t.toLowerCase()))) {
      queries.push(`"${marketTitle}" - on-chain data, institutional flows, and technical levels`);
    } else if (category === "sports" || alertContext.tags?.some(t => ["sports", "nba", "nfl"].includes(t.toLowerCase()))) {
      queries.push(`"${marketTitle}" - injury updates, lineup changes, and sharp money movement`);
    } else if (category === "politics" || alertContext.tags?.some(t => ["politics", "election"].includes(t.toLowerCase()))) {
      queries.push(`"${marketTitle}" - latest polling data, demographic shifts, and political developments`);
    } else {
      queries.push(`"${marketTitle}" - expert predictions and probability assessment for this outcome`);
    }
  }

  return queries;
}

/**
 * Build a structured research signal from Perplexity results + alert context.
 * Synthesizes research into an actionable signal.
 */
export function buildResearchSignal(
  perplexityResults: PerplexityResponse[],
  score: AlertScore,
  alertContext: {
    action: string;
    outcome?: string | null;
    value: number;
    winRate?: number | null;
    rank?: number | null;
    portfolio?: number | null;
    bidImbalance?: number;
    isHeavyActor?: boolean;
  }
): ResearchSignal {
  // Determine direction from whale action + score tier
  let direction: "bullish" | "bearish" | "neutral";
  if (alertContext.action === "BUY") {
    direction = "bullish";
  } else if (alertContext.action === "SELL") {
    direction = "bearish";
  } else {
    direction = "neutral";
  }

  // Map score tier to confidence
  const confidence = score.tier;

  // Build whale quality string
  const qualityParts: string[] = [];
  if (alertContext.rank !== undefined && alertContext.rank !== null) {
    qualityParts.push(`Rank #${alertContext.rank}`);
  }
  if (alertContext.winRate !== undefined && alertContext.winRate !== null) {
    qualityParts.push(`${(alertContext.winRate * 100).toFixed(0)}% win rate`);
  }
  if (alertContext.portfolio !== undefined && alertContext.portfolio !== null) {
    qualityParts.push(`$${formatCompact(alertContext.portfolio)} portfolio`);
  }
  if (alertContext.isHeavyActor) {
    qualityParts.push("heavy actor");
  }
  const whale_quality = qualityParts.length > 0
    ? qualityParts.join(", ")
    : "Unknown trader";

  // Build market pressure string
  let market_pressure = "No order book data";
  if (alertContext.bidImbalance !== undefined) {
    const bidPct = (alertContext.bidImbalance * 100).toFixed(0);
    const askPct = ((1 - alertContext.bidImbalance) * 100).toFixed(0);
    if (alertContext.bidImbalance >= 0.6) {
      market_pressure = `Bid pressure (${bidPct}/${askPct} bid/ask)`;
    } else if (alertContext.bidImbalance <= 0.4) {
      market_pressure = `Ask pressure (${bidPct}/${askPct} bid/ask)`;
    } else {
      market_pressure = `Balanced (${bidPct}/${askPct} bid/ask)`;
    }
  }

  // Summarize research (combine successful Perplexity answers)
  const successfulAnswers = perplexityResults
    .filter((r) => !r.error && r.answer)
    .map((r) => r.answer);

  let research_summary: string;
  if (successfulAnswers.length === 0) {
    research_summary = "No research data available.";
  } else {
    // Take first ~300 chars from each answer to build a digest
    const digest = successfulAnswers
      .map((a) => {
        const sentences = a.split(/[.!?]+/).filter((s) => s.trim());
        return sentences.slice(0, 2).join(". ").trim();
      })
      .filter((d) => d)
      .join(". ");
    research_summary = digest.length > 500 ? digest.slice(0, 497) + "..." : digest;
  }

  return {
    direction,
    confidence,
    factors: score.factors,
    whale_quality,
    market_pressure,
    research_summary,
  };
}

/**
 * Run multiple Perplexity searches for comprehensive research
 */
export async function runResearchQueries(
  marketTitle: string,
  apiKey: string,
  category?: string,
  customQueries?: string[]
): Promise<PerplexitySearchResult> {
  const queries = customQueries || generateResearchQueries(marketTitle, category);

  const results: PerplexityResponse[] = [];

  for (const query of queries) {
    const result = await queryPerplexity(query, apiKey);
    results.push(result);

    // Small delay to avoid rate limiting
    await new Promise(resolve => setTimeout(resolve, 500));
  }

  return {
    queries,
    results,
  };
}

function formatCompact(v: number): string {
  if (v >= 1_000_000) return `${(v / 1_000_000).toFixed(1)}M`;
  if (v >= 1_000) return `${(v / 1_000).toFixed(0)}k`;
  return v.toFixed(0);
}
