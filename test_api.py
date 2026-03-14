import os
import sys

# Add the project root to sys.path so we can import src
sys.path.append(os.path.dirname(os.path.abspath(__file__)))

from src.aggregator import MarketAggregator
from src.utils import logger

def run_test():
    agg = MarketAggregator()
    logger.info("Fetching markets...")
    markets = agg.fetch_all_markets()
    
    k_len = len(markets.get("kalshi", []))
    p_len = len(markets.get("polymarket", []))
    
    logger.info(f"Kalshi returned {k_len} markets.")
    logger.info(f"Polymarket returned {p_len} markets.")
    
    if k_len > 0:
        logger.info(f"Sample Kalshi: {markets['kalshi'][0].get('ticker')}")
    if p_len > 0:
        logger.info(f"Sample Polymarket: {markets['polymarket'][0].get('title')}")

if __name__ == "__main__":
    run_test()
