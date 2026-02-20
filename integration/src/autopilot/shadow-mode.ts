/**
 * Shadow Autopilot
 * Automatically "trades" whale alerts in simulation for testing.
 * Limits to 500 trades and logs P&L.
 */

import * as fs from "fs";
import * as path from "path";
import { WhalertAlert } from "../util/types.js";
import { EnvConfig } from "../util/env.js";
import { scoreAlert } from "../scoring/scorer.js";
import { reconcileTrades } from "../util/settlement.js";

export interface ShadowState {
    tradesCount: number;
    totalPnL: number;
    activeTrades: any[];
    maxTrades: number;
}

export class ShadowAutopilot {
    private statePath: string;
    private logPath: string;
    private csvPath: string;
    private state: ShadowState;

    constructor(private env: EnvConfig) {
        const homeDir = process.env.HOME || "";
        const configDir = path.join(homeDir, ".config", "wwatcher");
        if (!fs.existsSync(configDir)) fs.mkdirSync(configDir, { recursive: true });

        this.statePath = path.join(configDir, "shadow_state.json");
        this.logPath = path.join(configDir, "shadow_trades.jsonl");

        // CSV moved to project root for easier access
        const projectRoot = path.join(homeDir, "Documents", "PROJECTS", "polymaster");
        const tradeLogsDir = path.join(projectRoot, "trade_logs");
        if (!fs.existsSync(tradeLogsDir)) fs.mkdirSync(tradeLogsDir, { recursive: true });

        this.csvPath = path.join(tradeLogsDir, "shadow_trades.csv");
        this.state = this.loadState();

        // Initialize CSV headers if file is new
        if (!fs.existsSync(this.csvPath)) {
            const headers = "timestamp,platform,market_title,action,price,score,tier,win_rate,factors\n";
            fs.writeFileSync(this.csvPath, headers);
        }
    }

    /**
     * Start the autopilot background tasks
     */
    start(): void {
        console.log("[Shadow] Starting background reconciliation loop (every 5m)...");
        // Initial check
        this.reconcile().catch(err => console.error("[Shadow] Settlement error:", err));

        // Periodically reconcile
        setInterval(() => {
            this.reconcile().catch(err => console.error("[Shadow] Settlement error:", err));
        }, 5 * 60 * 1000); // 5 minutes
    }

    private loadState(): ShadowState {
        if (fs.existsSync(this.statePath)) {
            try {
                return JSON.parse(fs.readFileSync(this.statePath, "utf-8"));
            } catch {
                // Fallback to default
            }
        }
        return { tradesCount: 0, totalPnL: 0, activeTrades: [], maxTrades: 10000 };
    }

    private saveState(): void {
        fs.writeFileSync(this.statePath, JSON.stringify(this.state, null, 2));
    }

    private logTrade(entry: any): void {
        fs.appendFileSync(this.logPath, JSON.stringify(entry) + "\n");
        this.logTradeCSV(entry);
    }

    private logTradeCSV(entry: any): void {
        const row = [
            entry.timestamp,
            entry.platform,
            `"${(entry.alert_market || "").replace(/"/g, '""')}"`,
            entry.action,
            entry.entry_price,
            entry.score,
            entry.tier,
            entry.win_rate,
            `"${(entry.factors || []).join(" | ").replace(/"/g, '""')}"`
        ].join(",");
        fs.appendFileSync(this.csvPath, row + "\n");
    }

    /**
     * Reconcile current trades with market outcomes
     */
    async reconcile(): Promise<void> {
        this.state.activeTrades = await reconcileTrades(this.state.activeTrades);

        // Update stats
        this.state.totalPnL = this.state.activeTrades
            .filter(t => t.status === "SETTLED")
            .reduce((sum, t) => sum + (t.pnl || 0), 0);

        this.saveState();
    }

    /**
     * Main entry point for processing a new whale alert
     */
    async processAlert(alert: WhalertAlert): Promise<void> {
        if (this.state.tradesCount >= this.state.maxTrades) {
            console.log(`[Shadow] Max trades reached (${this.state.maxTrades}). Skipping.`);
            return;
        }

        // Step 1: Score the alert
        const score = scoreAlert(alert);
        const wr = alert.whale_profile?.win_rate
            ? `${(alert.whale_profile.win_rate * 100).toFixed(0)}%`
            : "N/A";

        console.log(`[Shadow] Received alert: ${alert.market_title} | WR: ${wr} | Score: ${score.score} | Tier: ${score.tier}`);

        // Removed tier filtering to copy "all whales" for pattern analysis
        console.log(`[Shadow] Copying signal (Direct Log Mode) for pattern analysis...`);

        try {
            // Step 2: Execute Simulated Trade Instantly
            this.state.tradesCount++;
            const trade = {
                id: `shadow-${Date.now()}`,
                timestamp: new Date().toISOString(),
                alert_market: alert.market_title,
                target_ticker: alert.platform === "Kalshi" ? alert.market_title : "RAW_SIGNAL",
                platform: alert.platform,
                entry_price: alert.price,
                action: alert.action,
                status: "LOGGED",
                score: score.score,
                tier: score.tier,
                factors: score.factors,
                win_rate: wr
            };

            this.state.activeTrades.push(trade);
            this.saveState();
            this.logTrade(trade);

            console.log(`[Shadow] ✅ LOGGED SIMULATED TRADE #${this.state.tradesCount}: ${alert.market_title} @ ${alert.price} (Score: ${score.score})`);
        } catch (err) {
            console.error(`[Shadow] Error logging alert:`, err);
        }
    }

    getStatus(): any {
        return {
            trades_executed: this.state.tradesCount,
            total_pnl: this.state.totalPnL.toFixed(4),
            remaining: this.state.maxTrades - this.state.tradesCount,
            active: this.state.activeTrades.length,
            recent_trades: this.state.activeTrades.slice(-3)
        };
    }
}
