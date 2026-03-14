from datetime import datetime, timedelta, timezone
from dateutil import parser
from src.aggregator import MarketAggregator
from src.utils import logger

class MarketScanner:
    def __init__(self):
        self.aggregator = MarketAggregator()
        self.MIN_VOLUME = 200
        self.MAX_EXPIRY_DAYS = 30

    def _parse_date(self, date_str):
        if not date_str:
            return None
        try:
            return parser.isoparse(date_str)
        except Exception:
            return None

    def _normalize_kalshi(self, market):
        """Converts Kalshi market to standard format."""
        try:
            close_date = self._parse_date(market.get("close_time"))
            volume = market.get("volume", 0)
            
            # Kalshi provides yes_ask, yes_bid
            yes_ask = market.get("yes_ask", 0)
            yes_bid = market.get("yes_bid", 0)
            spread = abs(yes_ask - yes_bid) if yes_ask and yes_bid else 0
            
            p_price = yes_ask if yes_ask > 0 else 50
            
            return {
                "id": market.get("ticker"),
                "platform": "kalshi",
                "title": market.get("title", ""),
                "volume": volume,
                "close_date": close_date,
                "price": p_price,
                "spread": spread,
                "raw_data": market
            }
        except Exception as e:
            logger.debug(f"Failed to normalize kalshi market: {e}")
            return None

    def _normalize_poly(self, event):
        """Converts Polymarket event to standard format."""
        # Polymarket events contain multiple markets. 
        # For simplicity, we create one item per event if there is active volume.
        try:
            close_date = self._parse_date(event.get("endDate"))
            volume = float(event.get("volume", 0))
            
            markets = event.get("markets", [])
            # Try to find a spread from the primary market
            spread = 0
            price = 50
            if markets:
                pm = markets[0]
                price = pm.get("outcomePrices", ["0.5", "0.5"])[0] 
                try:
                    price = float(price) * 100
                except ValueError:
                    price = 50
                    
            return {
                "id": event.get("id"),
                "platform": "polymarket",
                "title": event.get("title", ""),
                "volume": volume,
                "close_date": close_date,
                "price": price,
                "spread": spread,  # Needs CLOB orderbook for real spread
                "raw_data": event
            }
        except Exception as e:
            logger.debug(f"Failed to normalize poly market: {e}")
            return None

    def scan(self):
        """Fetch all markets, normalize, filter based on PRD bounds, return candidates."""
        logger.info("Starting scan...")
        raw_markets = self.aggregator.fetch_all_markets()
        
        candidates = []
        now = datetime.now(timezone.utc)
        
        # Process Kalshi
        for m in raw_markets.get("kalshi", []):
            norm = self._normalize_kalshi(m)
            if not norm: continue
            
            if norm["volume"] < self.MIN_VOLUME:
                continue
            if not norm["close_date"] or (norm["close_date"] - now).days > self.MAX_EXPIRY_DAYS:
                continue
                
            norm["anomaly_flag"] = "wide_spread" if norm["spread"] > 5 else None
            candidates.append(norm)
            
        # Process Polymarket
        for e in raw_markets.get("polymarket", []):
            norm = self._normalize_poly(e)
            if not norm: continue
            
            if norm["volume"] < self.MIN_VOLUME:
                continue
            if not norm["close_date"] or (norm["close_date"] - now).days > self.MAX_EXPIRY_DAYS:
                continue
                
            norm["anomaly_flag"] = None
            candidates.append(norm)

        logger.info(f"Scan complete. Found {len(candidates)} valid candidate markets.")
        return candidates

if __name__ == "__main__":
    scanner = MarketScanner()
    res = scanner.scan()
    for r in res[:5]:
        print(f"[{r['platform']}] {r['title']} - Vol: {r['volume']}")
