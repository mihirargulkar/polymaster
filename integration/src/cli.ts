#!/usr/bin/env node
/**
 * wwatcher-ai CLI — OpenClaw-compatible interface for whale alert research
 *
 * Commands:
 *   status                     Health check: history file, alert count, providers, API keys
 *   alerts [options]           Query recent alerts with filters
 *   summary                    Aggregate stats: volume, top markets, whale counts
 *   search <query>             Search alerts by market title/outcome text
 *   fetch <market_title>       Fetch RapidAPI data for a market (crypto/sports/weather/news)
 *   perplexity <query>         Run a single Perplexity search
 *   research <market_title>    Full research: RapidAPI + Perplexity + prediction markets
 *   score <alert_json>         Score a whale alert, return tier + factors
 *   preferences show           Show current alert preferences schema
 */

import * as fs from "fs";
import * as path from "path";
import * as readline from "node:readline/promises";
import { stdin as input, stdout as output } from "node:process";
import { fileURLToPath } from "url";
import { AlertStore } from "./watcher/alert-store.js";
import { ProviderRegistry } from "./providers/registry.js";
import { fetchAutoFromProvider } from "./providers/fetcher.js";
import { queryPerplexity, runResearchQueries, generateResearchQueries, generateContextQueries, buildResearchSignal } from "./providers/perplexity.js";
import { runFreeResearch, checkMarketEquivalence } from "./providers/free-research.js";
import { fetchPredictionMarketData } from "./providers/prediction-fetcher.js";
import { scoreAlert, passesPreferences } from "./scoring/scorer.js";
import { loadEnv } from "./util/env.js";
import { placeKalshiOrder, getKalshiBalance, searchKalshiMarkets } from "./tools/kalshi-execution.js";
import type { WhalertAlert, AlertPreferences } from "./util/types.js";

interface CliOptions {
  limit?: number;
  platform?: string;
  alertType?: string;
  minValue?: number;
  since?: string;
  category?: string;
  queries?: number;
  context?: string;
  json?: boolean;
}

function parseArgs(args: string[]): { command: string; positional: string[]; options: CliOptions } {
  const command = args[0] || "help";
  const positional: string[] = [];
  const options: CliOptions = {};

  for (let i = 1; i < args.length; i++) {
    const arg = args[i];
    if (arg.startsWith("--")) {
      const eqIdx = arg.indexOf("=");
      const key = eqIdx >= 0 ? arg.slice(2, eqIdx) : arg.slice(2);
      const value = eqIdx >= 0 ? arg.slice(eqIdx + 1) : args[++i];
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
        case "queries":
        case "q":
          options.queries = parseInt(value, 10);
          break;
        case "context":
          options.context = value;
          break;
        case "json":
          options.json = true;
          i--; // no value consumed
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
        case "q":
          options.queries = parseInt(value, 10);
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
  status                     Health check: history file, alert count, providers, API keys
  alerts                     Query recent alerts with filters
  summary                    Aggregate stats: volume, top markets, whale counts
  search <query>             Search alerts by market title/outcome text
  fetch <market_title>       Fetch RapidAPI data for a market
  perplexity <query>         Run a single Perplexity search
  research <market_title>    Full research: RapidAPI + Perplexity + prediction markets
  score <alert_json>         Score a whale alert JSON, return tier + factors
  preferences show           Show alert preferences schema and example
  balance                    Get Kalshi account balance
  buy <ticker> <qty> [px]    Place Kalshi buy (px in cents for limit)
  sell <ticker> <qty> [px]   Place Kalshi sell (px in cents for limit)
  settle-shadow              Reconcile shadow trades with real outcomes

ALERT OPTIONS:
  --limit=N, -l N            Max alerts to return (default: 20)
  --platform=X, -p X         Filter by platform (polymarket, kalshi)
  --type=X, -t X             Filter by alert type (WHALE_ENTRY, WHALE_EXIT)
  --min=N, -m N              Minimum transaction value in USD
  --since=ISO, -s ISO        Only alerts after this timestamp

FETCH/RESEARCH OPTIONS:
  --category=X, -c X         Override category (weather, crypto, sports, news, politics)
  --queries=N, -q N          Number of Perplexity queries for research (default: 3)
  --context=JSON              Alert JSON for context-aware research (auto-scores + targeted queries)

EXAMPLES:
  wwatcher-ai status
  wwatcher-ai alerts --limit=10 --min=50000
  wwatcher-ai fetch "Bitcoin price above 100k"
  wwatcher-ai research "Bitcoin above 100k by March" --category=crypto
  wwatcher-ai research "Bitcoin above 100k" --context='{"action":"BUY","value":50000,...}'
  wwatcher-ai score '{"platform":"polymarket","action":"BUY","value":50000,...}'
  wwatcher-ai preferences show
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

  // Try to load history from Rust binary first
  const binaryPath = path.join(fileURLToPath(new URL(".", import.meta.url)), "../../target/release/wwatcher");
  if (fs.existsSync(binaryPath)) {
    store.loadFromBinary(binaryPath);
  } else {
    // Fallback to legacy JSONL if binary not found
    store.loadFromFile(env.historyPath);
  }

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
          categories: registry.categories(),
          list: providers,
        },
        api_keys: {
          rapidapi: !!env.rapidApiKey,
          perplexity: !!env.perplexityApiKey,
        },
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
          market_context: a.market_context,
          whale_profile: a.whale_profile,
          order_book: a.order_book,
          top_holders: a.top_holders,
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
          help: "Set RAPIDAPI_KEY in integration/.env",
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

    case "perplexity": {
      const query = positional.join(" ");
      if (!query) {
        console.error(JSON.stringify({ error: "Query required. Usage: wwatcher-ai perplexity <query>" }));
        process.exit(1);
      }

      if (!env.perplexityApiKey) {
        console.log(JSON.stringify({
          error: "PERPLEXITY_API_KEY not configured",
          help: "Set PERPLEXITY_API_KEY in integration/.env. Get your key at https://perplexity.ai/settings/api",
        }, null, 2));
        process.exit(1);
      }

      const result = await queryPerplexity(query, env.perplexityApiKey);
      console.log(JSON.stringify({
        query: result.query,
        answer: result.answer,
        citations: result.citations,
        error: result.error,
      }, null, 2));
      break;
    }

    case "score": {
      const alertJson = positional.join(" ");
      if (!alertJson) {
        console.error(JSON.stringify({ error: "Alert JSON required. Usage: wwatcher-ai score '<alert_json>'" }));
        process.exit(1);
      }

      let alert: WhalertAlert;
      try {
        alert = JSON.parse(alertJson) as WhalertAlert;
      } catch {
        console.error(JSON.stringify({ error: "Invalid JSON. Provide a valid whale alert JSON object." }));
        process.exit(1);
      }

      const alertScore = scoreAlert(alert);
      console.log(JSON.stringify(alertScore, null, 2));
      break;
    }

    case "preferences": {
      const subcommand = positional[0] || "show";
      if (subcommand === "show") {
        const schema: AlertPreferences = {
          min_value: 100000,
          min_win_rate: 0.6,
          max_leaderboard_rank: 100,
          platforms: ["polymarket"],
          categories: ["crypto", "politics"],
          directions: ["buy"],
          tier_filter: "high",
        };
        console.log(JSON.stringify({
          description: "Alert preferences schema. Store in OpenClaw memory under key 'wwatcher_preferences'. All fields are optional — only set what you want to filter on.",
          example: schema,
          fields: {
            min_value: "Minimum trade value in USD (e.g. 100000 = skip trades under $100k)",
            min_win_rate: "Minimum whale win rate as decimal (e.g. 0.6 = 60%+). Polymarket only.",
            max_leaderboard_rank: "Maximum leaderboard rank (e.g. 100 = top 100 traders only). Polymarket only.",
            platforms: "Array of platforms to include (e.g. ['polymarket'])",
            categories: "Array of market categories (e.g. ['crypto', 'politics']). Matches against tags.",
            directions: "Array of trade directions (e.g. ['buy'] = entries only, ['sell'] = exits only)",
            tier_filter: "Minimum alert tier after scoring ('high' or 'medium'). Skips low-tier alerts.",
          },
        }, null, 2));
      } else {
        console.error(JSON.stringify({ error: `Unknown preferences subcommand: ${subcommand}. Use: preferences show` }));
        process.exit(1);
      }
      break;
    }

    case "research": {
      const marketTitle = positional.join(" ");
      if (!marketTitle) {
        console.error(JSON.stringify({ error: "Market title required. Usage: wwatcher-ai research <market_title>" }));
        process.exit(1);
      }

      if (!env.perplexityApiKey && !(env.tavilyApiKey && env.groqApiKey)) {
        console.log(JSON.stringify({
          error: "No research API keys configured",
          help: "Research requires either PERPLEXITY_API_KEY OR (TAVILY_API_KEY and GROQ_API_KEY) in integration/.env",
        }, null, 2));
        process.exit(1);
      }

      // Parse optional alert context for context-aware research
      let alertContext: WhalertAlert | null = null;
      if (options.context) {
        try {
          alertContext = JSON.parse(options.context) as WhalertAlert;
        } catch {
          console.error(JSON.stringify({ error: "Invalid --context JSON" }));
          process.exit(1);
        }
      }

      // Step 1: Fetch RapidAPI data (if key configured)
      const matches = registry.match(marketTitle, options.category);
      let rapidApiResults: any[] = [];

      if (env.rapidApiKey && matches.length > 0) {
        const providersToFetch = options.category
          ? matches
          : matches.filter((m) => !m.provider.match_all || matches.length === 1);

        // Exclude prediction-markets category from RapidAPI fetch (uses direct fetcher)
        const rapidProviders = providersToFetch.filter((m) => m.provider.category !== "prediction-markets");

        if (rapidProviders.length > 0) {
          rapidApiResults = await Promise.all(
            rapidProviders.map((m) =>
              fetchAutoFromProvider(m.provider, marketTitle, env.rapidApiKey!)
            )
          );
        }
      }

      // Step 2: Fetch prediction market data (no API key needed)
      const predictionData = await fetchPredictionMarketData(
        marketTitle,
        alertContext?.platform || "polymarket"
      );

      // Step 3: Run Perplexity searches (context-aware if alert provided)
      let perplexityResults;
      let alertScore = null;
      let signal = null;

      if (alertContext) {
        // Context-aware path: score the alert, generate targeted queries
        alertScore = scoreAlert(alertContext);

        // Compute bid imbalance
        let bidImbalance: number | undefined;
        if (alertContext.order_book) {
          const total = alertContext.order_book.bid_depth_10pct + alertContext.order_book.ask_depth_10pct;
          if (total > 0) bidImbalance = alertContext.order_book.bid_depth_10pct / total;
        }

        // Detect contrarian position
        const yesPrice = alertContext.market_context?.yes_price || alertContext.price;
        const isContrarian =
          (alertContext.outcome?.toLowerCase() === "no" && alertContext.action === "BUY" && yesPrice > 0.6) ||
          (alertContext.outcome?.toLowerCase() === "yes" && alertContext.action === "BUY" && yesPrice < 0.4);

        const contextQueries = generateContextQueries(marketTitle, alertScore, {
          action: alertContext.action,
          outcome: alertContext.outcome,
          value: alertContext.value,
          platform: alertContext.platform,
          winRate: alertContext.whale_profile?.win_rate,
          rank: alertContext.whale_profile?.leaderboard_rank,
          bidImbalance,
          isHeavyActor: alertContext.wallet_activity?.is_heavy_actor,
          isContrarian,
          tags: alertContext.market_context?.tags,
        });

        if (env.perplexityApiKey) {
          perplexityResults = await runResearchQueries(
            marketTitle,
            env.perplexityApiKey,
            options.category,
            contextQueries
          );
        } else {
          perplexityResults = await runFreeResearch(
            marketTitle,
            env.tavilyApiKey!,
            env.groqApiKey!,
            options.category,
            contextQueries
          );
        }

        // Build structured signal
        signal = buildResearchSignal(perplexityResults.results, alertScore, {
          action: alertContext.action,
          outcome: alertContext.outcome,
          value: alertContext.value,
          winRate: alertContext.whale_profile?.win_rate,
          rank: alertContext.whale_profile?.leaderboard_rank,
          portfolio: alertContext.whale_profile?.portfolio_value,
          bidImbalance,
          isHeavyActor: alertContext.wallet_activity?.is_heavy_actor,
        });
      } else {
        // Legacy path: generic research queries
        const numQueries = options.queries || 5;
        const queries = generateResearchQueries(marketTitle, options.category).slice(0, numQueries);
        if (env.perplexityApiKey) {
          perplexityResults = await runResearchQueries(
            marketTitle,
            env.perplexityApiKey,
            options.category,
            queries
          );
        } else {
          perplexityResults = await runFreeResearch(
            marketTitle,
            env.tavilyApiKey!,
            env.groqApiKey!,
            options.category,
            queries
          );
        }
      }

      // Step 4: Cross-Platform Equivalence (if Polymarket alert)
      let equivalence = null;
      if (alertContext && alertContext.platform.toLowerCase() === "polymarket" && env.groqApiKey) {
        try {
          // Find best matching Kalshi market
          const kalshiMatches = await searchKalshiMarkets(marketTitle, 1);
          if (kalshiMatches.length > 0) {
            const bestMatch = kalshiMatches[0];
            equivalence = await checkMarketEquivalence(
              { title: marketTitle },
              { title: bestMatch.title, subtitle: bestMatch.subtitle },
              env.groqApiKey!
            );
            // Attach the ticker for the quick trade command if equivalent
            if (equivalence.isEquivalent) {
              (equivalence as any).target_ticker = bestMatch.ticker;
            }
          }
        } catch (err) {
          console.error("Warning: Equivalence check failed:", err);
        }
      }

      // Step 5: Compile research report
      const report: any = {
        market_title: marketTitle,
        category: options.category || (matches[0]?.provider.category ?? "general"),
        timestamp: new Date().toISOString(),
      };

      if (equivalence) {
        report.cross_platform_equivalence = equivalence;
      }

      if (alertScore) {
        report.alert_score = alertScore;
      }

      if (signal) {
        report.signal = signal;
      }

      report.prediction_market_data = {
        related_markets: predictionData.related_markets || [],
        cross_platform: predictionData.cross_platform || null,
        price_history_points: predictionData.price_history?.length || 0,
      };

      if (rapidApiResults.length > 0) {
        report.rapidapi_data = {
          providers_matched: matches
            .filter((m) => m.provider.category !== "prediction-markets")
            .map((m) => ({
              name: m.provider.name,
              category: m.provider.category,
            })),
          results: rapidApiResults.map((r) => ({
            provider: r.provider,
            status: r.status,
            data: r.data,
            error: r.error,
          })),
        };
      }

      report.perplexity_research = {
        queries_run: perplexityResults.queries.length,
        context_aware: !!alertContext,
        results: perplexityResults.results.map((r) => ({
          query: r.query,
          answer: r.answer,
          citations: r.citations,
          error: r.error,
        })),
      };

      report.research_summary = {
        data_sources: (rapidApiResults.length > 0 ? matches.length : 0) + perplexityResults.queries.length + 1,
        rapidapi_providers: rapidApiResults.length > 0 ? matches.filter((m) => m.provider.category !== "prediction-markets").length : 0,
        perplexity_queries: perplexityResults.queries.length,
        prediction_market_data: !!(predictionData.related_markets?.length || predictionData.cross_platform),
        successful_queries: perplexityResults.results.filter(r => !r.error).length,
      };

      console.log(JSON.stringify(report, null, 2));
      break;
    }

    case "balance": {
      try {
        const balance = await getKalshiBalance(env);
        console.log(JSON.stringify({ platform: "kalshi", balance, currency: "USD" }, null, 2));
      } catch (err) {
        console.error(JSON.stringify({ error: err instanceof Error ? err.message : String(err) }));
        process.exit(1);
      }
      break;
    }

    case "search-kalshi": {
      const query = positional.join(" ");
      if (!query) {
        console.error(JSON.stringify({ error: "Usage: wwatcher-ai search-kalshi <query>" }));
        process.exit(1);
      }
      try {
        const results = await searchKalshiMarkets(query);
        console.log(JSON.stringify(results, null, 2));
      } catch (err) {
        console.error(JSON.stringify({ error: err instanceof Error ? err.message : String(err) }));
        process.exit(1);
      }
      break;
    }

    case "trade": {
      // Interactive Trade Cockpit
      const recentAlerts = store.query({ limit: 5 });
      if (recentAlerts.length === 0) {
        console.log("No recent alerts found in history. Try 'wwatcher watch' first.");
        break;
      }

      console.log("\n--- 🚀 TRADE COCKPIT ---");
      recentAlerts.forEach((a, i) => {
        console.log(`[${i + 1}] ${a.platform} | ${a.action} ${a.outcome} | $${Number(a.value).toLocaleString()} | ${a.market_title}`);
      });

      const rl = readline.createInterface({ input, output });
      try {
        const choice = await rl.question("\nSelect alert to trade [1-5]: ");
        const idx = parseInt(choice) - 1;

        if (isNaN(idx) || idx < 0 || idx >= recentAlerts.length) {
          console.log("Invalid selection.");
          break;
        }

        const alert = recentAlerts[idx];
        console.log(`\nSelected: ${alert.market_title}`);

        // Ensure Kalshi ticker resolution
        let ticker = alert.market_title || "";
        if (alert.platform === "Kalshi") {
          console.log("Resolving ticker...");
          const searchResults = await searchKalshiMarkets(ticker, 1);
          if (searchResults.length > 0) {
            ticker = searchResults[0].ticker;
            console.log(`Resolved to: ${ticker}`);
          }
        }

        const qtyStr = await rl.question("Quantity [default 10]: ");
        const count = parseInt(qtyStr) || 10;

        const confirm = await rl.question(`Confirm ${alert.action} ${count} shares of ${ticker}? [y/N]: `);
        if (confirm.toLowerCase() !== "y") {
          console.log("Aborted.");
          break;
        }

        if (alert.platform === "Kalshi") {
          const result = await placeKalshiOrder(env, {
            ticker,
            action: alert.action.toLowerCase() as "buy" | "sell",
            side: "yes",
            count,
            type: "market"
          });
          console.log("Trade Success!", JSON.stringify(result, null, 2));
        } else {
          console.log("Polymarket execution not yet implemented in CLI. Use Kalshi for now.");
        }
      } catch (err) {
        console.error("Error during trade:", err);
      } finally {
        rl.close();
      }
      break;
    }

    case "buy":
    case "sell": {
      let ticker = positional[0];
      const count = parseInt(positional[1], 10);
      const price = parseInt(positional[2], 10); // Cents

      if (!ticker || isNaN(count)) {
        console.error(JSON.stringify({ error: `Usage: wwatcher-ai ${command} <ticker_or_search> <count> [price_cents]` }));
        process.exit(1);
      }

      // If ticker looks like a search query (contains spaces or no dashes), try to resolve it
      if (ticker.includes(" ") || (!ticker.includes("-") && ticker.length > 5)) {
        console.error(JSON.stringify({ status: "searching", query: ticker }));
        try {
          const searchResults = await searchKalshiMarkets(ticker, 1);
          if (searchResults.length === 0) {
            console.error(JSON.stringify({ error: `No market found for query: ${ticker}` }));
            process.exit(1);
          }
          ticker = searchResults[0].ticker;
          console.error(JSON.stringify({ status: "resolved", ticker, title: searchResults[0].title }));
        } catch (err) {
          console.error(JSON.stringify({ error: "Failed to resolve market search" }));
          process.exit(1);
        }
      }

      try {
        const result = await placeKalshiOrder(env, {
          ticker,
          action: command as "buy" | "sell",
          side: "yes",
          count,
          type: price ? "limit" : "market",
          ...(price && { price }),
        });
        console.log(JSON.stringify({ status: "success", result, ticker }, null, 2));
      } catch (err: any) {
        console.error(JSON.stringify({
          error: err.response?.data?.error?.message || err.message,
          details: err.response?.data
        }));
        process.exit(1);
      }
      break;
    }

    case "settle-shadow": {
      const { ShadowAutopilot } = await import("./autopilot/shadow-mode.js");
      const bot = new ShadowAutopilot(env);
      console.log("🔍 Reconciling shadow trades with market outcomes...");
      await bot.reconcile();
      const status = bot.getStatus();
      console.log("✅ Settlement complete.");
      console.log(JSON.stringify(status, null, 2));
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
