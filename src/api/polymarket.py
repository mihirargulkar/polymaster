import requests
from src.utils import logger

class PolymarketClient:
    def __init__(self):
        # Polymarket Gamma API for discovery
        self.base_url = "https://gamma-api.polymarket.com"
        
    def get_markets(self, limit=100):
        """Fetch active markets from Polymarket via Gamma API."""
        try:
            params = {
                "active": "true",
                "closed": "false",
                "limit": limit
            }
            resp = requests.get(f"{self.base_url}/events", params=params)
            resp.raise_for_status()
            return resp.json()
        except Exception as e:
            logger.error(f"Error fetching Polymarket markets: {e}")
            if 'resp' in locals():
                logger.error(resp.text)
            return []
