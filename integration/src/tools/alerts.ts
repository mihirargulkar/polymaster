import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import type { AlertStore } from "../watcher/alert-store.js";

/** Register alert query tools: get_recent_alerts, get_alert_summary, search_alerts */
export function registerAlertTools(server: McpServer, store: AlertStore): void {
  // get_recent_alerts — query alert history with filters
  server.tool(
    "get_recent_alerts",
    "Query whale alert history with filters (limit, platform, alert_type, min_value, since)",
    {
      limit: z.number().optional().default(20).describe("Max number of alerts to return"),
      platform: z.string().optional().describe("Filter by platform (polymarket, kalshi)"),
      alert_type: z.string().optional().describe("Filter by type (WHALE_ENTRY, WHALE_EXIT)"),
      min_value: z.number().optional().describe("Minimum transaction value in USD"),
      since: z.string().optional().describe("ISO timestamp — only alerts after this time"),
    },
    async (params) => {
      const alerts = store.query({
        limit: params.limit,
        platform: params.platform,
        alert_type: params.alert_type,
        min_value: params.min_value,
        since: params.since,
      });

      return {
        content: [
          {
            type: "text" as const,
            text: JSON.stringify(
              {
                count: alerts.length,
                alerts: alerts.map((a) => ({
                  platform: a.platform,
                  alert_type: a.alert_type,
                  action: a.action,
                  value: a.value,
                  price_percent: a.price_percent,
                  market_title: a.market_title,
                  outcome: a.outcome,
                  timestamp: a.timestamp,
                  wallet_id: a.wallet_id,
                  wallet_activity: a.wallet_activity,
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

  // get_alert_summary — aggregate stats
  server.tool(
    "get_alert_summary",
    "Aggregate stats: total volume, breakdown by platform/market/action, top markets, whale counts",
    {},
    async () => {
      const summary = store.summarize();

      return {
        content: [
          {
            type: "text" as const,
            text: JSON.stringify(summary, null, 2),
          },
        ],
      };
    }
  );

  // search_alerts — text search in market titles
  server.tool(
    "search_alerts",
    "Search alerts by text in market title or outcome",
    {
      query: z.string().describe("Search text to match against market titles and outcomes"),
      limit: z.number().optional().default(20).describe("Max results to return"),
    },
    async (params) => {
      const results = store.search(params.query, params.limit);

      return {
        content: [
          {
            type: "text" as const,
            text: JSON.stringify(
              {
                query: params.query,
                count: results.length,
                alerts: results,
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
