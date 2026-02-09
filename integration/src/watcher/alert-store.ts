import * as fs from "fs";
import type { WhalertAlert, AlertFilter, AlertSummary } from "../util/types.js";

/**
 * In-memory store for whale alerts. Loads history at startup,
 * receives live alerts from the file watcher.
 */
export class AlertStore {
  private alerts: WhalertAlert[] = [];

  /** Load existing alerts from the JSONL history file */
  loadFromFile(filePath: string): void {
    if (!fs.existsSync(filePath)) return;

    const content = fs.readFileSync(filePath, "utf-8");
    for (const line of content.split("\n")) {
      const trimmed = line.trim();
      if (!trimmed) continue;
      try {
        const alert = JSON.parse(trimmed) as WhalertAlert;
        this.alerts.push(alert);
      } catch {
        // Skip malformed lines
      }
    }
  }

  /** Add a single alert (called by file watcher on new lines) */
  addAlert(alert: WhalertAlert): void {
    this.alerts.push(alert);
  }

  /** Get the byte offset to start tailing from (end of current file) */
  get count(): number {
    return this.alerts.length;
  }

  get latestAlertTime(): string | null {
    if (this.alerts.length === 0) return null;
    return this.alerts[this.alerts.length - 1].timestamp;
  }

  /** Query alerts with filters */
  query(filter: AlertFilter): WhalertAlert[] {
    let results = [...this.alerts];

    if (filter.platform) {
      const p = filter.platform.toLowerCase();
      results = results.filter(
        (a) => a.platform.toLowerCase() === p
      );
    }

    if (filter.alert_type) {
      const t = filter.alert_type.toUpperCase();
      results = results.filter((a) => a.alert_type === t);
    }

    if (filter.min_value !== undefined) {
      results = results.filter((a) => a.value >= filter.min_value!);
    }

    if (filter.since) {
      const sinceDate = new Date(filter.since).getTime();
      results = results.filter(
        (a) => new Date(a.timestamp).getTime() >= sinceDate
      );
    }

    // Most recent first
    results.reverse();

    if (filter.limit !== undefined && filter.limit > 0) {
      results = results.slice(0, filter.limit);
    }

    return results;
  }

  /** Search alerts by text in market_title or outcome */
  search(query: string, limit: number = 20): WhalertAlert[] {
    const q = query.toLowerCase();
    const results = this.alerts
      .filter(
        (a) =>
          (a.market_title && a.market_title.toLowerCase().includes(q)) ||
          (a.outcome && a.outcome.toLowerCase().includes(q))
      )
      .reverse()
      .slice(0, limit);
    return results;
  }

  /** Generate aggregate summary stats */
  summarize(): AlertSummary {
    const byPlatform: Record<string, number> = {};
    const byAction: Record<string, number> = {};
    const marketMap: Record<string, { count: number; volume: number }> = {};
    let totalVolume = 0;
    let repeatActors = 0;
    let heavyActors = 0;
    const seenWallets = new Set<string>();

    for (const alert of this.alerts) {
      totalVolume += alert.value;

      byPlatform[alert.platform] = (byPlatform[alert.platform] || 0) + 1;
      byAction[alert.action] = (byAction[alert.action] || 0) + 1;

      const title = alert.market_title || "Unknown";
      if (!marketMap[title]) {
        marketMap[title] = { count: 0, volume: 0 };
      }
      marketMap[title].count++;
      marketMap[title].volume += alert.value;

      if (alert.wallet_activity && alert.wallet_id) {
        if (!seenWallets.has(alert.wallet_id)) {
          seenWallets.add(alert.wallet_id);
          if (alert.wallet_activity.is_repeat_actor) repeatActors++;
          if (alert.wallet_activity.is_heavy_actor) heavyActors++;
        }
      }
    }

    const topMarkets = Object.entries(marketMap)
      .map(([title, data]) => ({ title, ...data }))
      .sort((a, b) => b.volume - a.volume)
      .slice(0, 10);

    return {
      total_alerts: this.alerts.length,
      total_volume: totalVolume,
      by_platform: byPlatform,
      by_action: byAction,
      top_markets: topMarkets,
      whale_count: { repeat_actors: repeatActors, heavy_actors: heavyActors },
      latest_alert_time: this.latestAlertTime,
    };
  }
}
