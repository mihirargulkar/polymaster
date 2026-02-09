#!/usr/bin/env node

import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { loadEnv } from "./util/env.js";
import { createServer } from "./server.js";
import { FileWatcher } from "./watcher/file-watcher.js";

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  const mode = args.includes("--mode=snapshot") ? "snapshot" : "realtime";

  // Load environment configuration
  const env = loadEnv();

  // Create MCP server with all tools
  const { server, store } = createServer({
    historyPath: env.historyPath,
    providersConfigPath: env.providersConfigPath,
    rapidApiKey: env.rapidApiKey,
  });

  // Start real-time file watcher if not in snapshot mode
  let watcher: FileWatcher | undefined;
  if (mode === "realtime") {
    watcher = new FileWatcher(env.historyPath, store);
    watcher.start();
  }

  // Connect via stdio transport
  const transport = new StdioServerTransport();
  await server.connect(transport);

  // Graceful shutdown
  const shutdown = (): void => {
    watcher?.stop();
    process.exit(0);
  };

  process.on("SIGINT", shutdown);
  process.on("SIGTERM", shutdown);
}

main().catch((err) => {
  console.error("Fatal error:", err);
  process.exit(1);
});
