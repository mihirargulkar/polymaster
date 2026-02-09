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
}

export interface WalletActivity {
  transactions_last_hour: number;
  transactions_last_day: number;
  total_value_hour: number;
  total_value_day: number;
  is_repeat_actor: boolean;
  is_heavy_actor: boolean;
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
}

/** Alert summary stats */
export interface AlertSummary {
  total_alerts: number;
  total_volume: number;
  by_platform: Record<string, number>;
  by_action: Record<string, number>;
  top_markets: Array<{ title: string; count: number; volume: number }>;
  whale_count: { repeat_actors: number; heavy_actors: number };
  latest_alert_time: string | null;
}
