/** Matches the JSON schema from wwatcher's build_alert_payload() in src/alerts/mod.rs */
export interface WhalertAlert {
  platform: string;
  alert_type: "WHALE_ENTRY" | "WHALE_EXIT";
  action: "BUY" | "SELL";
  value: number;
  price: number;
  price_percent: number;
  size: number;
  timestamp: string;
  market_title: string | null;
  outcome: string | null;
  wallet_id?: string;
  wallet_activity?: WalletActivity;
  market_context?: MarketContext;
  whale_profile?: WhaleProfile;
  order_book?: OrderBook;
  top_holders?: TopHolders;
}

export interface WalletActivity {
  transactions_last_hour: number;
  transactions_last_day: number;
  total_value_hour: number;
  total_value_day: number;
  is_repeat_actor: boolean;
  is_heavy_actor: boolean;
}

/** Market context from Gamma/Kalshi API — odds, volume, spread, tags */
export interface MarketContext {
  yes_price: number;
  no_price: number;
  spread: number;
  volume_24h: number;
  open_interest: number;
  price_change_24h: number;
  liquidity: number;
  tags: string[];
}

/** Whale profile from Polymarket Data API — portfolio, rank, win rate */
export interface WhaleProfile {
  portfolio_value?: number | null;
  leaderboard_rank?: number | null;
  leaderboard_profit?: number | null;
  win_rate?: number | null;
  positions_count?: number | null;
  markets_traded?: number | null;
}

/** Order book depth from CLOB/Kalshi orderbook API */
export interface OrderBook {
  best_bid: number;
  best_ask: number;
  bid_depth_10pct: number;
  ask_depth_10pct: number;
  bid_levels: number;
  ask_levels: number;
}

/** Top holders per market from Polymarket Data API */
export interface TopHolders {
  holders: TopHolder[];
  total_shares: number;
}

export interface TopHolder {
  wallet: string;
  shares: number;
  value: number;
}

/** Provider definitions loaded from providers.json */
export interface ProviderParam {
  type: "string" | "number";
  required: boolean;
  description: string;
  default?: string | number;
}

export interface ProviderEndpoint {
  method: "GET" | "POST";
  path: string;
  description: string;
  params: Record<string, ProviderParam>;
}

export interface Provider {
  name: string;
  category: string;
  rapidapi_host: string;
  env_key: string;
  keywords: string[];
  match_all?: boolean;
  endpoints: Record<string, ProviderEndpoint>;
}

export type ProvidersConfig = Record<string, Provider>;

/** wwatcher config.json schema (from src/config.rs) */
export interface WwatcherConfig {
  kalshi_api_key_id?: string;
  kalshi_private_key?: string;
  webhook_url?: string;
}

/** Alert query filters */
export interface AlertFilter {
  limit?: number;
  platform?: string;
  alert_type?: string;
  min_value?: number;
  since?: string;
  tags?: string[];
  min_win_rate?: number;
  max_leaderboard_rank?: number;
}

/** Alert summary stats */
export interface AlertSummary {
  total_alerts: number;
  total_volume: number;
  by_platform: Record<string, number>;
  by_action: Record<string, number>;
  top_markets: Array<{ title: string; count: number; volume: number }>;
  whale_count: { repeat_actors: number; heavy_actors: number };
  avg_whale_rank: number | null;
  avg_bid_depth: number | null;
  latest_alert_time: string | null;
}

/** Alert scoring types */
export interface AlertScore {
  score: number;
  tier: "high" | "medium" | "low";
  factors: string[];
}

/** Structured research signal output */
export interface ResearchSignal {
  direction: "bullish" | "bearish" | "neutral";
  confidence: "high" | "medium" | "low";
  factors: string[];
  whale_quality: string;
  market_pressure: string;
  research_summary: string;
}

/** User-defined alert preferences (stored in OpenClaw memory) */
export interface AlertPreferences {
  min_value?: number;
  min_win_rate?: number;
  max_leaderboard_rank?: number;
  platforms?: string[];
  categories?: string[];
  directions?: string[];
  tier_filter?: "high" | "medium";
}
