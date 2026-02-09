import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { AlertStore } from "./watcher/alert-store.js";
import { ProviderRegistry } from "./providers/registry.js";
import { registerAlertTools } from "./tools/alerts.js";
import { registerMarketDataTool } from "./tools/market-data.js";
import { registerStatusTool } from "./tools/status.js";

export interface ServerConfig {
  historyPath: string;
  providersConfigPath: string;
  rapidApiKey: string | undefined;
}

/** Create and configure the MCP server with all tools registered */
export function createServer(config: ServerConfig): {
  server: McpServer;
  store: AlertStore;
  registry: ProviderRegistry;
} {
  const server = new McpServer({
    name: "wwatcher",
    version: "1.0.0",
  });

  // Initialize the alert store and load existing history
  const store = new AlertStore();
  store.loadFromFile(config.historyPath);

  // Initialize the provider registry
  const registry = new ProviderRegistry(config.providersConfigPath);

  // Register all tools
  registerAlertTools(server, store);
  registerMarketDataTool(server, registry, config.rapidApiKey);
  registerStatusTool(server, store, registry, config.historyPath, config.rapidApiKey);

  return { server, store, registry };
}
