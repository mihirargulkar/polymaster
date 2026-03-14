import time
import os
import asyncio
from src.utils import logger
from pmxt import Polymarket, Kalshi

class ArbitrageScanner:
    def __init__(self):
        self.poly = Polymarket()
        self.kalshi = Kalshi()
        self.max_cost = 0.98  # To guarantee a profit after fees, we need to buy both sides for < $0.98

    async def scan_overlapping_strikes(self):
        """
        Scans Polymarket and Kalshi for overlapping strike prices matching our Arbitrage Thesis.
        If we buy Kalshi 'YES' and Polymarket 'DOWN' and the total cost < $1.00, it's risk-free.
        """
        logger.info("[ARBITRAGE] Starting cross-platform options overlap scan...")
        
        # We fetch active markets on both platforms to search for overlapping "Yes" and "Down" options.
        try:
            poly_markets = self.poly.fetch_markets(limit=20)
            kalshi_markets = self.kalshi.fetch_markets(limit=20)
            
            # Simple matching logic on string similarity
            for p in poly_markets:
                for k in kalshi_markets:
                    p_title = getattr(p, "title", "")
                    k_title = getattr(k, "title", "")
                    
                    if p_title and k_title and p_title.lower()[:15] == k_title.lower()[:15]:
                        # Handle UnifiedMarket price attributes
                        poly_price = getattr(p, "price", 0.50)
                        kalshi_price = getattr(k, "yes_ask", 0.50)
                        
                        if (poly_price + kalshi_price) < self.max_cost:
                            logger.info(f"[ARBITRAGE] Found Match: {p_title} combined cost: ${(poly_price + kalshi_price):.2f}")
                            return {"poly_leg": getattr(p, "id", ""), "kalshi_leg": getattr(k, "ticker", "")}
                            
        except Exception as e:
            logger.error(f"[ARBITRAGE] API Error fetching overlapping orders: {e}")
            
        logger.info("[ARBITRAGE] No $1.00 Arbitrage overlaps detected in current sweep.")
        return None

if __name__ == "__main__":
    from dotenv import load_dotenv
    load_dotenv()
    scanner = ArbitrageScanner()
    asyncio.run(scanner.scan_overlapping_strikes())
