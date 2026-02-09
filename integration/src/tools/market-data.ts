import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import type { ProviderRegistry } from "../providers/registry.js";
import { fetchAutoFromProvider } from "../providers/fetcher.js";

/**
 * Register the fetch_market_data tool.
 * Takes a market_title + optional category → matches keywords in providers.json →
 * calls relevant RapidAPI endpoints → returns structured data.
 */
export function registerMarketDataTool(
  server: McpServer,
  registry: ProviderRegistry,
  rapidApiKey: string | undefined
): void {
  server.tool(
    "fetch_market_data",
    "Fetch contextual market data from RapidAPI providers. Matches market title keywords to find relevant data sources (crypto prices, sports odds, weather forecasts, news). Returns structured data for AI analysis.",
    {
      market_title: z
        .string()
        .describe("The market title or description to find relevant data for"),
      category: z
        .string()
        .optional()
        .describe("Override category (weather, crypto, sports, news) instead of auto-matching"),
    },
    async (params) => {
      if (!rapidApiKey) {
        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify(
                {
                  error: "RAPIDAPI_KEY not configured",
                  help: "Set RAPIDAPI_KEY in integration/.env or as an environment variable. Get your key at https://rapidapi.com",
                  matched_providers: registry
                    .match(params.market_title, params.category)
                    .map((m) => ({
                      provider: m.provider.name,
                      category: m.provider.category,
                      matched_keywords: m.matchedKeywords,
                    })),
                },
                null,
                2
              ),
            },
          ],
        };
      }

      const matches = registry.match(params.market_title, params.category);

      if (matches.length === 0) {
        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify(
                {
                  market_title: params.market_title,
                  message: "No matching providers found for this market title",
                  available_providers: registry.list(),
                },
                null,
                2
              ),
            },
          ],
        };
      }

      // Fetch from all matching providers (excluding match_all like news unless explicitly requested)
      const providersToFetch = params.category
        ? matches
        : matches.filter((m) => !m.provider.match_all || matches.length === 1);

      const results = await Promise.all(
        providersToFetch.map((m) =>
          fetchAutoFromProvider(m.provider, params.market_title, rapidApiKey)
        )
      );

      return {
        content: [
          {
            type: "text" as const,
            text: JSON.stringify(
              {
                market_title: params.market_title,
                providers_matched: matches.map((m) => ({
                  name: m.provider.name,
                  category: m.provider.category,
                  keywords_matched: m.matchedKeywords,
                })),
                results: results.map((r) => ({
                  provider: r.provider,
                  endpoint: r.endpoint,
                  status: r.status,
                  error: r.error,
                  data: r.data,
                })),
              },
              null,
              2
            ),
          },
        ],
      };
    }
  );
}
