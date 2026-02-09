import * as fs from "fs";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import type { AlertStore } from "../watcher/alert-store.js";
import type { ProviderRegistry } from "../providers/registry.js";

/** Register the get_wwatcher_status health check tool */
export function registerStatusTool(
  server: McpServer,
  store: AlertStore,
  registry: ProviderRegistry,
  historyPath: string,
  rapidApiKey: string | undefined
): void {
  server.tool(
    "get_wwatcher_status",
    "Health check: history file status, alert count, latest alert time, configured providers, API key status",
    {},
    async () => {
      const historyExists = fs.existsSync(historyPath);
      let historySize = 0;
      if (historyExists) {
        historySize = fs.statSync(historyPath).size;
      }

      const providers = registry.list();

      return {
        content: [
          {
            type: "text" as const,
            text: JSON.stringify(
              {
                status: "running",
                history_file: {
                  path: historyPath,
                  exists: historyExists,
                  size_bytes: historySize,
                },
                alerts: {
                  total_loaded: store.count,
                  latest_alert_time: store.latestAlertTime,
                },
                providers: {
                  count: providers.length,
                  list: providers,
                },
                api_key_configured: !!rapidApiKey,
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
