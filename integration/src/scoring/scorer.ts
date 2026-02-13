import type { WhalertAlert, AlertScore, AlertPreferences } from "../util/types.js";

/**
 * Score a whale alert based on enriched data.
 * Higher score = more significant/actionable alert.
 *
 * Factors weighted:
 * - Whale leaderboard rank (top 50 = high, top 500 = medium)
 * - Win rate (>70% = high conviction trader)
 * - Heavy/repeat actor status
 * - Trade size relative to threshold
 * - Order book imbalance (directional pressure)
 * - Contrarian position (buying NO when YES is dominant)
 */
export function scoreAlert(alert: WhalertAlert): AlertScore {
  let score = 0;
  const factors: string[] = [];

  // --- Whale profile signals ---
  const rank = alert.whale_profile?.leaderboard_rank;
  if (rank !== undefined && rank !== null) {
    if (rank <= 10) {
      score += 30;
      factors.push(`Top 10 leaderboard trader (#${rank})`);
    } else if (rank <= 50) {
      score += 25;
      factors.push(`Top 50 leaderboard trader (#${rank})`);
    } else if (rank <= 100) {
      score += 20;
      factors.push(`Top 100 leaderboard trader (#${rank})`);
    } else if (rank <= 500) {
      score += 10;
      factors.push(`Top 500 leaderboard trader (#${rank})`);
    }
  }

  const winRate = alert.whale_profile?.win_rate;
  if (winRate !== undefined && winRate !== null) {
    if (winRate >= 0.8) {
      score += 20;
      factors.push(`Elite win rate (${(winRate * 100).toFixed(0)}%)`);
    } else if (winRate >= 0.7) {
      score += 15;
      factors.push(`Strong win rate (${(winRate * 100).toFixed(0)}%)`);
    } else if (winRate >= 0.6) {
      score += 10;
      factors.push(`Above average win rate (${(winRate * 100).toFixed(0)}%)`);
    }
  }

  const portfolio = alert.whale_profile?.portfolio_value;
  if (portfolio !== undefined && portfolio !== null) {
    if (portfolio >= 1_000_000) {
      score += 10;
      factors.push(`Large portfolio ($${formatValue(portfolio)})`);
    } else if (portfolio >= 500_000) {
      score += 5;
      factors.push(`Mid-size portfolio ($${formatValue(portfolio)})`);
    }
  }

  // --- Wallet activity signals ---
  if (alert.wallet_activity?.is_heavy_actor) {
    score += 15;
    factors.push(`Heavy actor (${alert.wallet_activity.transactions_last_day} txns/24h)`);
  } else if (alert.wallet_activity?.is_repeat_actor) {
    score += 10;
    factors.push(`Repeat actor (${alert.wallet_activity.transactions_last_hour} txns/1h)`);
  }

  // --- Trade size signal ---
  if (alert.value >= 250_000) {
    score += 20;
    factors.push(`Massive trade ($${formatValue(alert.value)})`);
  } else if (alert.value >= 100_000) {
    score += 15;
    factors.push(`Large trade ($${formatValue(alert.value)})`);
  } else if (alert.value >= 50_000) {
    score += 10;
    factors.push(`Significant trade ($${formatValue(alert.value)})`);
  } else {
    score += 5;
    factors.push(`Trade size: $${formatValue(alert.value)}`);
  }

  // --- Order book imbalance signal ---
  if (alert.order_book) {
    const totalDepth = alert.order_book.bid_depth_10pct + alert.order_book.ask_depth_10pct;
    if (totalDepth > 0) {
      const bidPct = alert.order_book.bid_depth_10pct / totalDepth;
      if (bidPct >= 0.65) {
        score += 10;
        factors.push(`Strong bid pressure (${(bidPct * 100).toFixed(0)}% bid)`);
      } else if (bidPct <= 0.35) {
        score += 10;
        factors.push(`Strong ask pressure (${((1 - bidPct) * 100).toFixed(0)}% ask)`);
      }
    }
  }

  // --- Contrarian signal ---
  if (alert.market_context) {
    const yesPrice = alert.market_context.yes_price;
    const isBuyingNo = alert.outcome?.toLowerCase() === "no" && alert.action === "BUY";
    const isBuyingYes = alert.outcome?.toLowerCase() === "yes" && alert.action === "BUY";

    if (isBuyingNo && yesPrice > 0.6) {
      score += 15;
      factors.push(`Contrarian: buying NO when YES is at ${(yesPrice * 100).toFixed(0)}%`);
    } else if (isBuyingYes && yesPrice < 0.4) {
      score += 15;
      factors.push(`Contrarian: buying YES when YES is at ${(yesPrice * 100).toFixed(0)}%`);
    }
  }

  // --- Determine tier ---
  let tier: "high" | "medium" | "low";
  if (score >= 60) {
    tier = "high";
  } else if (score >= 35) {
    tier = "medium";
  } else {
    tier = "low";
  }

  return { score, tier, factors };
}

/**
 * Check if an alert passes user-defined preference filters.
 * Returns true if the alert should be processed.
 */
export function passesPreferences(alert: WhalertAlert, prefs: AlertPreferences): boolean {
  if (prefs.min_value !== undefined && alert.value < prefs.min_value) {
    return false;
  }

  if (prefs.min_win_rate !== undefined) {
    const wr = alert.whale_profile?.win_rate;
    if (wr === undefined || wr === null || wr < prefs.min_win_rate) {
      return false;
    }
  }

  if (prefs.max_leaderboard_rank !== undefined) {
    const rank = alert.whale_profile?.leaderboard_rank;
    if (rank === undefined || rank === null || rank > prefs.max_leaderboard_rank) {
      return false;
    }
  }

  if (prefs.platforms && prefs.platforms.length > 0) {
    if (!prefs.platforms.includes(alert.platform.toLowerCase())) {
      return false;
    }
  }

  if (prefs.categories && prefs.categories.length > 0) {
    const tags = alert.market_context?.tags?.map((t) => t.toLowerCase()) || [];
    const hasMatch = prefs.categories.some((c) => tags.includes(c.toLowerCase()));
    if (!hasMatch) {
      return false;
    }
  }

  if (prefs.directions && prefs.directions.length > 0) {
    const action = alert.action.toLowerCase();
    if (!prefs.directions.includes(action)) {
      return false;
    }
  }

  return true;
}

function formatValue(v: number): string {
  if (v >= 1_000_000) return `${(v / 1_000_000).toFixed(1)}M`;
  if (v >= 1_000) return `${(v / 1_000).toFixed(0)}k`;
  return v.toFixed(0);
}
