import urllib.parse
import os
import requests
from src.utils import logger

class TwitterScraper:
    """
    Mock/Proxy Twitter Scraper.
    In a real production environment, this would hook into the X API (Basic/Pro) 
    or a scraping service like Apify. Since API keys are required and expensive, 
    we implement a mock version that simulates fetching recent tweets for testing NLP.
    """
    def __init__(self):
        self.api_key = os.getenv("TWITTER_BEARER_TOKEN")
        
    def fetch_recent_tweets(self, query, limit=10):
        logger.info(f"Fetching live Twitter data for query: {query}")
        
        if not self.api_key:
            logger.warning("TWITTER_BEARER_TOKEN is not set. Cannot fetch live tweets.")
            return []

        headers = {"Authorization": f"Bearer {self.api_key}"}
        url = f"https://api.twitter.com/2/tweets/search/recent?query={urllib.parse.quote(query)}"
        
        try:
            resp = requests.get(url, headers=headers)
            resp.raise_for_status()
            data = resp.json()
            
            live_tweets = []
            for item in data.get("data", [])[:limit]:
                live_tweets.append({
                    "author": item.get("author_id", "unknown"),
                    "text": item.get("text", "")
                })
            return live_tweets
        except Exception as e:
            logger.error(f"Failed to fetch live tweets: {e}")
            return []

if __name__ == "__main__":
    t = TwitterScraper()
    res = t.fetch_recent_tweets("Bitcoin")
    for r in res:
        print(f"@{r['author']}: {r['text']}")
