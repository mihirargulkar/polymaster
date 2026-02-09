#!/usr/bin/env node
/**
 * wwatcher-ai CLI — OpenClaw-compatible interface for whale alert research
 * 
 * Commands:
 *   status                     Health check: history file, alert count, providers, API key
 *   alerts [options]           Query recent alerts with filters
 *   summary                    Aggregate stats: volume, top markets, whale counts
 *   search <query>             Search alerts by market title/outcome text
 *   fetch <market_title>       Fetch RapidAPI data for a market (crypto/sports/weather/news)
 */

import * as fs from "fs";
import { AlertStore } from "./watcher/alert-store.js";
import { ProviderRegistry } from "./providers/registry.js";
import { fetchAutoFromProvider } from "./providers/fetcher.js";
import { loadEnv } from "./util/env.js";

interface CliOptions {
  limit?: number;
  platform?: string;
  alertType?: string;
  minValue?: number;
  since?: string;
  category?: string;
}

function parseArgs(args: string[]): { command: string; positional: string[]; options: CliOptions } {
  const command = args[0] || "help";
  const positional: string[] = [];
  const options: CliOptions = {};

  for (let i = 1; i < args.length; i++) {
    const arg = args[i];
    if (arg.startsWith("--")) {
      const [key, value] = arg.slice(2).split("=");
      switch (key) {
        case "limit":
          options.limit = parseInt(value, 10);
          break;
        case "platform":
          options.platform = value;
          break;
        case "alert-type":
        case "type":
          options.alertType = value;
          break;
        case "min-value":
        case "min":
          options.minValue = parseFloat(value);
          break;
        case "since":
          options.since = value;
          break;
        case "category":
        case "cat":
          options.category = value;
          break;
      }
    } else if (arg.startsWith("-")) {
      // Short flags
      const key = arg.slice(1);
      const value = args[++i];
      switch (key) {
        case "l":
          options.limit = parseInt(value, 10);
          break;
        case "p":
          options.platform = value;
          break;
        case "t":
          options.alertType = value;
          break;
        case "m":
          options.minValue = parseFloat(value);
          break;
        case "s":
          options.since = value;
          break;
        case "c":
          options.category = value;
          break;
      }
    } else {
      positional.push(arg);
    }
  }

  return { command, positional, options };
}

function printHelp(): void {
  console.log(`
wwatcher-ai — Whale Alert Research CLI for OpenClaw

USAGE:
  wwatcher-ai <command> [options]

COMMANDS:
  status                     Health check: history file, alert count, providers, API key
  alerts                     Query recent alerts with filters
  summary                    Aggregate stats: volume, top markets, whale counts
  search <query>             Search alerts by market title/outcome text
  fetch <market_title>       Fetch RapidAPI data for a market

ALERT OPTIONS:
  --limit=N, -l N            Max alerts to return (default: 20)
  --platform=X, -p X         Filter by platform (polymarket, kalshi)
  --type=X, -t X             Filter by alert type (WHALE_ENTRY, WHALE_EXIT)
  --min=N, -m N              Minimum transaction value in USD
  --since=ISO, -s ISO        Only alerts after this timestamp

FETCH OPTIONS:
  --category=X, -c X         Override category (weather, crypto, sports, news)

EXAMPLES:
  wwatcher-ai status
  wwatcher-ai alerts --limit=10 --min=50000
  wwatcher-ai alerts --platform=polymarket --type=WHALE_ENTRY
  wwatcher-ai summary
  wwatcher-ai search "bitcoin"
  wwatcher-ai fetch "Bitcoin price above 100k"
  wwatcher-ai fetch "Lakers vs Celtics" --category=sports
`);
}

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  const { command, positional, options } = parseArgs(args);

  if (command === "help" || command === "--help" || command === "-h") {
    printHelp();
    return;
  }

  // Load environment
  const env = loadEnv();

  // Initialize store and registry
  const store = new AlertStore();
  store.loadFromFile(env.historyPath);

  const registry = new ProviderRegistry(env.providersConfigPath);

  switch (command) {
    case "status": {
      const historyExists = fs.existsSync(env.historyPath);
      let historySize = 0;
      if (historyExists) {
        historySize = fs.statSync(env.historyPath).size;
      }

      const providers = registry.list();
      const result = {
        status: "running",
        history_file: {
          path: env.historyPath,
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
        api_key_configured: !!env.rapidApiKey,
      };
      console.log(JSON.stringify(result, null, 2));
      break;
    }

    case "alerts": {
      const alerts = store.query({
        limit: options.limit || 20,
        platform: options.platform,
        alert_type: options.alertType,
        min_value: options.minValue,
        since: options.since,
      });

      const result = {
        count: alerts.length,
        filters: {
          limit: options.limit || 20,
          platform: options.platform,
          alert_type: options.alertType,
          min_value: options.minValue,
          since: options.since,
        },
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
      };
      console.log(JSON.stringify(result, null, 2));
      break;
    }

    case "summary": {
      const summary = store.summarize();
      console.log(JSON.stringify(summary, null, 2));
      break;
    }

    case "search": {
      const query = positional.join(" ");
      if (!query) {
        console.error(JSON.stringify({ error: "Search query required. Usage: wwatcher-ai search <query>" }));
        process.exit(1);
      }

      const results = store.search(query, options.limit || 20);
      console.log(JSON.stringify({
        query,
        count: results.length,
        alerts: results,
      }, null, 2));
      break;
    }

    case "fetch": {
      const marketTitle = positional.join(" ");
      if (!marketTitle) {
        console.error(JSON.stringify({ error: "Market title required. Usage: wwatcher-ai fetch <market_title>" }));
        process.exit(1);
      }

      if (!env.rapidApiKey) {
        const matches = registry.match(marketTitle, options.category);
        console.log(JSON.stringify({
          error: "RAPIDAPI_KEY not configured",
          help: "Set RAPIDAPI_KEY in integration/.env or as environment variable. Get your key at https://rapidapi.com",
          matched_providers: matches.map((m) => ({
            provider: m.provider.name,
            category: m.provider.category,
            matched_keywords: m.matchedKeywords,
          })),
        }, null, 2));
        process.exit(1);
      }

      const matches = registry.match(marketTitle, options.category);

      if (matches.length === 0) {
        console.log(JSON.stringify({
          market_title: marketTitle,
          message: "No matching providers found for this market title",
          available_providers: registry.list(),
        }, null, 2));
        break;
      }

      // Fetch from matching providers (exclude match_all unless explicit category)
      const providersToFetch = options.category
        ? matches
        : matches.filter((m) => !m.provider.match_all || matches.length === 1);

      const results = await Promise.all(
        providersToFetch.map((m) =>
          fetchAutoFromProvider(m.provider, marketTitle, env.rapidApiKey!)
        )
      );

      console.log(JSON.stringify({
        market_title: marketTitle,
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
      }, null, 2));
      break;
    }

    default:
      console.error(JSON.stringify({ error: `Unknown command: ${command}`, help: "Run 'wwatcher-ai help' for usage" }));
      process.exit(1);
  }
}

main().catch((err) => {
  console.error(JSON.stringify({ error: err instanceof Error ? err.message : String(err) }));
  process.exit(1);
});
