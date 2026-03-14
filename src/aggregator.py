from src.api.kalshi import KalshiClient
from src.api.polymarket import PolymarketClient

class MarketAggregator:
    def __init__(self):
        self.kalshi = KalshiClient()
        self.poly = PolymarketClient()
        
    def fetch_all_markets(self):
        """Fetch active markets from all supported platforms."""
        kalshi_markets = self.kalshi.get_markets()
        poly_markets = self.poly.get_markets()
        
        return {
            "kalshi": kalshi_markets,
            "polymarket": poly_markets
        }
