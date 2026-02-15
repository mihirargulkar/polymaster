#!/usr/bin/env node

import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { loadEnv } from "./util/env.js";
import { createServer } from "./server.js";
import { FileWatcher } from "./watcher/file-watcher.js";
import { ShadowAutopilot } from "./autopilot/shadow-mode.js";

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  const mode = args.includes("--mode=snapshot") ? "snapshot" : (args.includes("--mode=shadow") ? "shadow" : "realtime");

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
  if (mode === "realtime" || mode === "shadow") {
    let onNewAlert;
    if (mode === "shadow") {
      const autopilot = new ShadowAutopilot(env);
      autopilot.start();
      console.log("🚀 Shadow Autopilot Active. Tracking 500 trades.");
      console.log(`Current Status: ${JSON.stringify(autopilot.getStatus(), null, 2)}`);
      onNewAlert = (alert: any) => autopilot.processAlert(alert);
    }

    watcher = new FileWatcher(env.historyPath, store, onNewAlert);
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
