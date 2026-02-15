import axios from "axios";
import { EnvConfig } from "./env.js";

export interface SettlementResult {
    ticker: string;
    outcome: "YES" | "NO" | "PENDING";
    payout: number; // 1 for win, 0 for loss, -1 for pending
}

/**
 * Fetches the outcome of a Kalshi market
 */
async function getKalshiOutcome(ticker: string): Promise<SettlementResult> {
    try {
        const response = await axios.get(`https://api.elections.kalshi.com/trade-api/v2/markets/${ticker}`);
        const market = response.data.market;

        if (market.status === "settled") {
            const outcome = market.result === "yes" ? "YES" : "NO";
            return { ticker, outcome, payout: market.result === "yes" ? 1 : 0 };
        }
        return { ticker, outcome: "PENDING", payout: -1 };
    } catch (err) {
        return { ticker, outcome: "PENDING", payout: -1 };
    }
}

/**
 * Fetches the outcome of a Polymarket market (simplified via title match search)
 * In a real scenario, we'd use the conditionId, but since we log raw signals,
 * we fuzzy match against closed markets.
 */
async function getPolymarketOutcome(title: string): Promise<SettlementResult> {
    try {
        // Polymarket doesn't have a simple "get by title" for closed markets easily
        // We'll tag it as pending for now until we implement a better lookup or 
        // rely on the user providing the outcome for the analysis phase.
        return { ticker: title, outcome: "PENDING", payout: -1 };
    } catch (err) {
        return { ticker: title, outcome: "PENDING", payout: -1 };
    }
}

/**
 * Reconcile a list of trades
 */
export async function reconcileTrades(trades: any[]): Promise<any[]> {
    const updated = [];
    for (const trade of trades) {
        if (trade.status === "SETTLED") {
            updated.push(trade);
            continue;
        }

        // Try to settle Kalshi trades if we have a ticker
        if (trade.platform === "Kalshi" && trade.target_ticker !== "RAW_SIGNAL") {
            const result = await getKalshiOutcome(trade.target_ticker);
            if (result.payout !== -1) {
                trade.status = "SETTLED";
                trade.outcome = result.outcome;
                trade.payout_received = result.payout;
                // Calculate PnL (Price is in 0-1 range)
                const isWin = (trade.action === "BUY" && trade.outcome === "YES") ||
                    (trade.action === "SELL" && trade.outcome === "NO");
                trade.pnl = isWin ? (1 - trade.entry_price) : -trade.entry_price;
            }
        }

        updated.push(trade);
    }
    return updated;
}
