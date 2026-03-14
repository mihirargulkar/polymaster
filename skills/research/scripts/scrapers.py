import urllib.parse
import feedparser
from src.utils import logger

class NewsScraper:
    """Uses Google News RSS to find recent articles on a topic."""
    
    def __init__(self):
        self.base_url = "https://news.google.com/rss/search?q={query}&hl=en-US&gl=US&ceid=US:en"
        
    def fetch_news(self, search_term, limit=5):
        encoded_query = urllib.parse.quote(search_term)
        url = self.base_url.format(query=encoded_query)
        logger.info(f"Fetching news for: {search_term}")
        
        try:
            feed = feedparser.parse(url)
            results = []
            
            for entry in feed.entries[:limit]:
                # Extract title, link, published date
                results.append({
                    "source": "Google News RSS",
                    "title": entry.get("title", ""),
                    "url": entry.get("link", ""),
                    "published": entry.get("published", "")
                })
            
            return results
        except Exception as e:
            logger.error(f"Failed to fetch Google News RSS for {search_term}: {e}")
            return []

if __name__ == "__main__":
    scraper = NewsScraper()
    news = scraper.fetch_news("MicroStrategy sells any Bitcoin")
    for n in news:
        print(f'- {n["title"]}\n  {n["url"]}\n')
