import axios from "axios";
import * as crypto from "crypto";
import { EnvConfig } from "../util/env.js";

/**
 * Kalshi API v2 Signing Helper
 */
function signKalshiRequest(
    method: string,
    path: string,
    timestamp: string,
    body: string = "",
    privateKey: string
): string {
    // Format: timestamp + method + path + body
    const payload = timestamp + method + path + body;

    const sign = crypto.createSign("RSA-SHA256");
    sign.update(payload);

    // Note: Kalshi expects base64 encoded signature
    return sign.sign(privateKey, "base64");
}

export interface KalshiOrderRequest {
    ticker: string;
    action: "buy" | "sell";
    side: "yes" | "no";
    count: number;
    type: "limit" | "market";
    price?: number; // Cents for Kalshi limit orders
}

/**
 * Places an order on Kalshi
 */
export async function placeKalshiOrder(
    env: EnvConfig,
    order: KalshiOrderRequest
): Promise<any> {
    if (!env.kalshiApiKeyId || !env.kalshiPrivateKey) {
        throw new Error("KALSHI_API_KEY_ID or KALSHI_PRIVATE_KEY is not configured");
    }

    const baseUrl = "https://api.elections.kalshi.com/trade-api/v2";
    const path = "/portfolio/orders";
    const method = "POST";
    const timestamp = Date.now().toString();

    const bodyObj = {
        ticker: order.ticker,
        action: order.action,
        side: order.side,
        count: order.count,
        type: order.type,
        ...(order.price && { price: order.price }),
        client_order_id: `pm-${Date.now()}`,
    };

    const bodyStr = JSON.stringify(bodyObj);
    const signature = signKalshiRequest(
        method,
        path,
        timestamp,
        bodyStr,
        env.kalshiPrivateKey
    );

    const response = await axios.post(`${baseUrl}${path}`, bodyObj, {
        headers: {
            "Content-Type": "application/json",
            "KALSHI-ACCESS-KEY": env.kalshiApiKeyId,
            "KALSHI-ACCESS-SIGNATURE": signature,
            "KALSHI-ACCESS-TIMESTAMP": timestamp,
        },
    });

    return response.data;
}

/**
 * Gets account balance from Kalshi
 */
export async function getKalshiBalance(env: EnvConfig): Promise<number> {
    if (!env.kalshiApiKeyId || !env.kalshiPrivateKey) {
        throw new Error("KALSHI_API_KEY_ID or KALSHI_PRIVATE_KEY is not configured");
    }

    const baseUrl = "https://api.elections.kalshi.com/trade-api/v2";
    const path = "/portfolio/balance";
    const method = "GET";
    const timestamp = Date.now().toString();

    const signature = signKalshiRequest(
        method,
        path,
        timestamp,
        "",
        env.kalshiPrivateKey
    );

    const response = await axios.get(`${baseUrl}${path}`, {
        headers: {
            "KALSHI-ACCESS-KEY": env.kalshiApiKeyId,
            "KALSHI-ACCESS-SIGNATURE": signature,
            "KALSHI-ACCESS-TIMESTAMP": timestamp,
        },
    });

    // Balance is returned in cents
    return response.data.balance / 100;
}

/**
 * Searches for Kalshi markets by title or query string
 */
export async function searchKalshiMarkets(
    query: string,
    limit: number = 5
): Promise<Array<{ ticker: string, title: string, subtitle: string }>> {
    const baseUrl = "https://api.elections.kalshi.com/trade-api/v2/markets";

    // Fetch active markets. We'll fetch 100 to find matches
    const response = await axios.get(baseUrl, {
        params: {
            status: "open",
            limit: 100
        }
    });

    const markets = response.data.markets || [];
    const q = query.toLowerCase();

    return markets
        .filter((m: any) =>
            (m.title && m.title.toLowerCase().includes(q)) ||
            (m.subtitle && m.subtitle.toLowerCase().includes(q)) ||
            (m.ticker && m.ticker.toLowerCase().includes(q))
        )
        .slice(0, limit)
        .map((m: any) => ({
            ticker: m.ticker,
            title: m.title,
            subtitle: m.subtitle
        }));
}
