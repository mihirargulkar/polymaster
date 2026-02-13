/**
 * Direct fetcher for prediction market APIs (Polymarket + Kalshi).
 * No API keys needed — all public endpoints.
 * Separate from the RapidAPI fetcher since these don't use X-RapidAPI headers.
 */

export interface PredictionMarketData {
  price_history?: PricePoint[];
  related_markets?: RelatedMarket[];
  cross_platform?: CrossPlatformMatch | null;
  error?: string;
}

export interface PricePoint {
  timestamp: string;
  price: number;
}

export interface RelatedMarket {
  title: string;
  outcome: string;
  price: number;
  volume?: number;
}

export interface CrossPlatformMatch {
  platform: string;
  ticker: string;
  title: string;
  yes_price: number;
  volume_24h?: number;
  open_interest?: number;
}

/**
 * Fetch prediction market data for an alert.
 * Calls Polymarket and/or Kalshi public APIs directly.
 */
export async function fetchPredictionMarketData(
  marketTitle: string,
  platform: string,
  conditionId?: string,
  tokenId?: string,
  ticker?: string
): Promise<PredictionMarketData> {
  const result: PredictionMarketData = {};

  try {
    if (platform.toLowerCase() === "polymarket") {
      // Fetch related markets from Gamma API by searching title
      const related = await fetchPolymarketRelated(marketTitle);
      if (related.length > 0) {
        result.related_markets = related;
      }

      // Fetch price history if we have a token ID
      if (tokenId) {
        const history = await fetchPolymarketPriceHistory(tokenId);
        if (history.length > 0) {
          result.price_history = history;
        }
      }

      // Try to find cross-platform match on Kalshi
      const kalshiMatch = await searchKalshiMarket(marketTitle);
      if (kalshiMatch) {
        result.cross_platform = kalshiMatch;
      }
    } else if (platform.toLowerCase() === "kalshi") {
      // Try to find cross-platform match on Polymarket
      const polyMatch = await searchPolymarketMarket(marketTitle);
      if (polyMatch) {
        result.cross_platform = polyMatch;
      }

      // Fetch related Kalshi markets by searching
      const related = await fetchKalshiRelated(marketTitle);
      if (related.length > 0) {
        result.related_markets = related;
      }
    }
  } catch (err) {
    result.error = err instanceof Error ? err.message : String(err);
  }

  return result;
}

/** Search Polymarket Gamma API for related markets */
async function fetchPolymarketRelated(marketTitle: string): Promise<RelatedMarket[]> {
  try {
    // Extract keywords for search (first 5 meaningful words)
    const keywords = marketTitle
      .replace(/[^a-zA-Z0-9\s]/g, "")
      .split(/\s+/)
      .filter((w) => w.length > 3)
      .slice(0, 3)
      .join(" ");

    if (!keywords) return [];

    const url = `https://gamma-api.polymarket.com/markets?_limit=5&active=true&closed=false&_q=${encodeURIComponent(keywords)}`;
    const resp = await fetch(url, { signal: AbortSignal.timeout(10000) });
    if (!resp.ok) return [];

    const markets = await resp.json() as any[];
    return markets
      .filter((m: any) => m.question && m.outcomePrices)
      .map((m: any) => {
        let price = 0;
        try {
          const prices = JSON.parse(m.outcomePrices);
          price = parseFloat(prices[0]) || 0;
        } catch { /* ignore */ }

        return {
          title: m.question,
          outcome: "Yes",
          price,
          volume: parseFloat(m.volume24hr) || 0,
        };
      })
      .slice(0, 5);
  } catch {
    return [];
  }
}

/** Search Polymarket for a specific market (used for cross-platform matching) */
async function searchPolymarketMarket(marketTitle: string): Promise<CrossPlatformMatch | null> {
  try {
    const keywords = marketTitle
      .replace(/[^a-zA-Z0-9\s]/g, "")
      .split(/\s+/)
      .filter((w) => w.length > 3)
      .slice(0, 3)
      .join(" ");

    if (!keywords) return null;

    const url = `https://gamma-api.polymarket.com/markets?_limit=1&active=true&_q=${encodeURIComponent(keywords)}`;
    const resp = await fetch(url, { signal: AbortSignal.timeout(10000) });
    if (!resp.ok) return null;

    const markets = await resp.json() as any[];
    if (markets.length === 0) return null;

    const m = markets[0];
    let yesPrice = 0;
    try {
      const prices = JSON.parse(m.outcomePrices);
      yesPrice = parseFloat(prices[0]) || 0;
    } catch { /* ignore */ }

    return {
      platform: "polymarket",
      ticker: m.conditionId || "",
      title: m.question || "",
      yes_price: yesPrice,
      volume_24h: parseFloat(m.volume24hr) || 0,
      open_interest: parseFloat(m.openInterest) || 0,
    };
  } catch {
    return null;
  }
}

/** Fetch Polymarket price history via CLOB API */
async function fetchPolymarketPriceHistory(tokenId: string): Promise<PricePoint[]> {
  try {
    const url = `https://clob.polymarket.com/prices-history?market=${tokenId}&interval=1d&fidelity=24`;
    const resp = await fetch(url, { signal: AbortSignal.timeout(10000) });
    if (!resp.ok) return [];

    const data = await resp.json() as any;
    const history = data.history || data;
    if (!Array.isArray(history)) return [];

    return history
      .map((p: any) => ({
        timestamp: p.t ? new Date(p.t * 1000).toISOString() : "",
        price: parseFloat(p.p) || 0,
      }))
      .filter((p: PricePoint) => p.timestamp && p.price > 0)
      .slice(-30); // Last 30 data points
  } catch {
    return [];
  }
}

/** Search Kalshi API for a matching market */
async function searchKalshiMarket(marketTitle: string): Promise<CrossPlatformMatch | null> {
  try {
    const keywords = marketTitle
      .replace(/[^a-zA-Z0-9\s]/g, "")
      .split(/\s+/)
      .filter((w) => w.length > 3)
      .slice(0, 3)
      .join(" ");

    if (!keywords) return null;

    const url = `https://api.elections.kalshi.com/trade-api/v2/markets?limit=1&status=open&title=${encodeURIComponent(keywords)}`;
    const resp = await fetch(url, { signal: AbortSignal.timeout(10000) });
    if (!resp.ok) return null;

    const data = await resp.json() as any;
    const markets = data.markets || [];
    if (markets.length === 0) return null;

    const m = markets[0];
    return {
      platform: "kalshi",
      ticker: m.ticker || "",
      title: m.title || "",
      yes_price: (m.yes_bid || m.last_price || 0) / 100,
      volume_24h: m.volume_24h,
      open_interest: m.open_interest,
    };
  } catch {
    return null;
  }
}

/** Fetch related Kalshi markets by searching */
async function fetchKalshiRelated(marketTitle: string): Promise<RelatedMarket[]> {
  try {
    const keywords = marketTitle
      .replace(/[^a-zA-Z0-9\s]/g, "")
      .split(/\s+/)
      .filter((w) => w.length > 3)
      .slice(0, 3)
      .join(" ");

    if (!keywords) return [];

    const url = `https://api.elections.kalshi.com/trade-api/v2/markets?limit=5&status=open&title=${encodeURIComponent(keywords)}`;
    const resp = await fetch(url, { signal: AbortSignal.timeout(10000) });
    if (!resp.ok) return [];

    const data = await resp.json() as any;
    const markets = data.markets || [];

    return markets.map((m: any) => ({
      title: m.title || "",
      outcome: "Yes",
      price: (m.yes_bid || m.last_price || 0) / 100,
      volume: m.volume_24h,
    }));
  } catch {
    return [];
  }
}
