# Integration Layer v2 Design

**Date**: 2026-02-13
**Status**: Complete (all 5 phases implemented)
**Scope**: Upgrade wwatcher-ai integration for OpenClaw — smarter research, structured signals, prediction market data, user preferences

---

## Problem Statement

The integration layer bridges Rust whale alerts to OpenClaw for AI-powered research. Three gaps existed:

1. **Data blindness** — TypeScript layer ignored whale profiles, order books, top holders, and tags from enriched Rust payloads
2. **Generic research** — Perplexity queries didn't leverage alert context (leaderboard rank, bid imbalance, position doubling)
3. **No automation** — Alerts didn't flow to OpenClaw automatically; the skill trigger existed but was basic

---

## Phase 1: Type Sync

- Added `MarketContext`, `WhaleProfile`, `OrderBook`, `TopHolders` interfaces to `types.ts`
- Added `AlertScore`, `ResearchSignal`, `AlertPreferences` types
- Updated `AlertFilter` with `tags`, `min_win_rate`, `max_leaderboard_rank`
- Updated `AlertSummary` with `avg_whale_rank`, `avg_bid_depth`
- Updated `AlertStore.query()` to filter on enriched fields
- Updated `AlertStore.search()` to search tags
- Updated `AlertStore.summarize()` to aggregate whale ranks and bid depth
- Updated MCP alert tools to return all enriched fields

## Phase 2: Alert Scoring

- New `src/scoring/scorer.ts` with `scoreAlert()` function
- Weighted scoring: whale rank, win rate, portfolio, wallet activity, trade size, order book imbalance, contrarian position
- Tier thresholds: high (>=60), medium (>=35), low (<35)
- `passesPreferences()` function for user-defined filters
- New CLI command: `score <alert_json>`

## Phase 3: Context-Aware Research

- New `generateContextQueries()` — builds 3 targeted Perplexity queries from alert score and context
- New `buildResearchSignal()` — synthesizes research into structured signal (direction, confidence, factors, whale quality, market pressure, summary)
- `research` command updated with `--context` flag for context-aware path
- Legacy generic research path preserved for backwards compatibility

## Phase 4: Prediction Market Provider

- New `src/providers/prediction-fetcher.ts` — direct HTTP to public Polymarket/Kalshi APIs
- No API keys needed
- Features: related markets search, cross-platform matching, price history
- New `providers/prediction-markets.json` config
- Integrated into `research` command pipeline

## Phase 5: SKILL.md + Preferences

- Updated `skill/SKILL.md` with full automated workflow
- Trigger patterns expanded (LARGE TRANSACTION DETECTED, WHALE ALERT, JSON patterns)
- User preferences system via OpenClaw memory (`wwatcher_preferences` key)
- Natural language preference examples
- Scoring reference table
- Structured signal delivery format
- New CLI command: `preferences show`

---

## Files

### New
- `src/scoring/scorer.ts` — Alert scoring and preference checking
- `src/providers/prediction-fetcher.ts` — Direct Polymarket/Kalshi API fetcher
- `providers/prediction-markets.json` — Prediction market provider config
- `docs/plans/2026-02-13-integration-v2-design.md` — This design doc

### Modified
- `src/util/types.ts` — Enriched interfaces, scoring types, preferences
- `src/watcher/alert-store.ts` — Enriched filtering, tag search, aggregation
- `src/tools/alerts.ts` — Enriched MCP tool responses, new filter params
- `src/providers/perplexity.ts` — Context queries, research signal builder
- `src/cli.ts` — New commands (score, preferences), --context flag, prediction data
- `skill/SKILL.md` — Full automated workflow, preferences, scoring reference

---

## CLI Commands (Final)

| Command | Purpose |
|---------|---------|
| `status` | Health check |
| `alerts` | Query alerts with enriched filters |
| `summary` | Aggregate stats with whale/depth metrics |
| `search <query>` | Text search (titles, outcomes, tags) |
| `fetch <title>` | RapidAPI data |
| `perplexity <query>` | Single Perplexity search |
| `research <title>` | Full research (RapidAPI + Perplexity + prediction markets) |
| `research <title> --context=<json>` | Context-aware research with scoring + structured signal |
| `score <json>` | Score alert, return tier + factors |
| `preferences show` | Show preference schema |
