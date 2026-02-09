import type { Provider, ProviderEndpoint } from "../util/types.js";

export interface FetchResult {
  provider: string;
  endpoint: string;
  status: number;
  data: unknown;
  error?: string;
}

/**
 * Generic RapidAPI fetcher. Reads provider config and builds the HTTP request.
 * Always sends X-RapidAPI-Key and X-RapidAPI-Host headers.
 */
export async function fetchFromProvider(
  provider: Provider,
  endpointKey: string,
  params: Record<string, string | number>,
  apiKey: string
): Promise<FetchResult> {
  const endpoint = provider.endpoints[endpointKey];
  if (!endpoint) {
    return {
      provider: provider.name,
      endpoint: endpointKey,
      status: 400,
      data: null,
      error: `Unknown endpoint "${endpointKey}" for provider "${provider.name}"`,
    };
  }

  const url = buildUrl(provider.rapidapi_host, endpoint, params);

  try {
    const response = await fetch(url, {
      method: endpoint.method,
      headers: {
        "X-RapidAPI-Key": apiKey,
        "X-RapidAPI-Host": provider.rapidapi_host,
      },
    });

    const data = await response.json();

    return {
      provider: provider.name,
      endpoint: endpointKey,
      status: response.status,
      data,
      error: response.ok ? undefined : `HTTP ${response.status}`,
    };
  } catch (err) {
    return {
      provider: provider.name,
      endpoint: endpointKey,
      status: 0,
      data: null,
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

/**
 * Build the full URL from provider config. Substitutes path params
 * like {latitude} and appends remaining params as query string.
 */
function buildUrl(
  host: string,
  endpoint: ProviderEndpoint,
  params: Record<string, string | number>
): string {
  let path = endpoint.path;
  const queryParams: Record<string, string> = {};

  // Substitute path params and collect query params
  for (const [key, value] of Object.entries(params)) {
    const placeholder = `{${key}}`;
    if (path.includes(placeholder)) {
      path = path.replace(placeholder, encodeURIComponent(String(value)));
    } else {
      queryParams[key] = String(value);
    }
  }

  // Add defaults for missing optional params
  for (const [key, paramDef] of Object.entries(endpoint.params)) {
    if (!params[key] && paramDef.default !== undefined) {
      queryParams[key] = String(paramDef.default);
    }
  }

  const base = `https://${host}${path}`;
  const qs = new URLSearchParams(queryParams).toString();
  return qs ? `${base}?${qs}` : base;
}

/**
 * Fetch from the first matching endpoint of a provider.
 * Picks the first endpoint and auto-fills params from the market title context.
 */
export async function fetchAutoFromProvider(
  provider: Provider,
  marketTitle: string,
  apiKey: string
): Promise<FetchResult> {
  const endpointKeys = Object.keys(provider.endpoints);
  if (endpointKeys.length === 0) {
    return {
      provider: provider.name,
      endpoint: "none",
      status: 400,
      data: null,
      error: "Provider has no endpoints",
    };
  }

  const endpointKey = endpointKeys[0];
  const endpoint = provider.endpoints[endpointKey];
  const params = extractParams(endpoint, marketTitle, provider.category);

  return fetchFromProvider(provider, endpointKey, params, apiKey);
}

/**
 * Best-effort parameter extraction from a market title.
 * For example, extracts "BTC" from a crypto market title,
 * or a sport key from a sports title.
 */
function extractParams(
  endpoint: ProviderEndpoint,
  marketTitle: string,
  category: string
): Record<string, string | number> {
  const params: Record<string, string | number> = {};
  const titleUpper = marketTitle.toUpperCase();

  for (const [key, paramDef] of Object.entries(endpoint.params)) {
    if (category === "crypto" && key === "symbol") {
      // Extract crypto symbols from title
      const symbols = ["BTC", "ETH", "SOL", "XRP", "ADA", "DOGE", "MATIC", "DOT"];
      const found = symbols.find((s) => titleUpper.includes(s));
      params[key] = found || "BTC";
    } else if (category === "sports" && key === "sport") {
      // Map sport keywords to API sport keys
      if (titleUpper.includes("NBA") || titleUpper.includes("BASKETBALL")) {
        params[key] = "basketball_nba";
      } else if (titleUpper.includes("NFL") || titleUpper.includes("FOOTBALL")) {
        params[key] = "americanfootball_nfl";
      } else if (titleUpper.includes("NHL") || titleUpper.includes("HOCKEY")) {
        params[key] = "icehockey_nhl";
      } else if (titleUpper.includes("MLB") || titleUpper.includes("BASEBALL")) {
        params[key] = "baseball_mlb";
      } else if (titleUpper.includes("SOCCER")) {
        params[key] = "soccer_epl";
      } else {
        params[key] = "basketball_nba";
      }
    } else if (category === "news" && key === "q") {
      // Use market title as search query (first 100 chars)
      params[key] = marketTitle.slice(0, 100);
    } else if (paramDef.default !== undefined) {
      params[key] = paramDef.default;
    }
  }

  return params;
}
