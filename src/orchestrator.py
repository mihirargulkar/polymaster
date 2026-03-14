import time
import os
import asyncio
from src.utils import logger
from src.scanner import MarketScanner
from skills.research.scripts.research import ResearcherAgent
from skills.research.scripts.scrapers import NewsScraper
from skills.research.scripts.twitter import TwitterScraper
from skills.predict.scripts.ensemble import PredictorAgent
from skills.predict_market_bot.scripts.validate_risk import RiskValidator
from src.arbitrage import ArbitrageScanner
from skills.compound.scripts.history import TradeLogger

# Set up dummy state for local simulation testing
class TradingBotOrchestrator:
    def __init__(self):
        self.scanner = MarketScanner()
        self.researcher = ResearcherAgent()
        self.news_scraper = NewsScraper()
        self.twitter_scraper = TwitterScraper()
        self.predictor = PredictorAgent()
        self.risk_manager = RiskValidator()
        self.arbitrage_scanner = ArbitrageScanner()
        self.trade_logger = TradeLogger()
        
        self.bankroll = 10000.0
        self.current_drawdown = 0.0
        self.daily_loss = 0.0
        self.concurrent_positions = 0
        self.daily_api_spend = 0.0
        self.api_budget_ceiling = 50.0 # From Beast Mode architecture

    def check_kill_switch(self):
        if os.path.exists("STOP"):
            logger.critical("KILL SWITCH ENGAGED! STOP file detected.")
            return True
        return False

    async def run_pipeline(self):
        logger.info("============== PIPELINE START ==============")
        
        if self.check_kill_switch():
            return
            
        # BULLETPROOF CHECK 1: Ensure AI budget is safe
        if self.daily_api_spend >= self.api_budget_ceiling:
            logger.warning(f"AI Budget exceeded (${self.daily_api_spend:.2f} >= ${self.api_budget_ceiling:.2f}). Sleeping till tomorrow.")
            await asyncio.sleep(86400) # Sleep for a day
            self.daily_api_spend = 0.0
            
        # BULLETPROOF CHECK 2: Arbitrage mathematical superiority
        arbs = await self.arbitrage_scanner.scan_overlapping_strikes()
        if arbs:
            logger.info(f"Executing Risk-Free Arbitrage instead of AI Prediction. Override triggered.")
            return

        # STEP 1: SCAN
        candidates = self.scanner.scan()
        if not candidates:
            logger.info("No candidate markets found.")
            return
            
        logger.info(f"Processing {len(candidates)} candidates.")
        
        for target in candidates:
            # Re-check kill switch in deep loop
            if self.check_kill_switch():
                break
                
            logger.info(f"Target selected: {target['title']} on {target['platform']}")
            
            # STEP 2: RESEARCH
            news = self.news_scraper.fetch_news(target['title'], limit=3)
            tweets = self.twitter_scraper.fetch_recent_tweets(target['title'], limit=3)
            brief = self.researcher.analyze(target['title'], news, tweets)
            
            logger.info(f"Research compiled.")
            
            # STEP 3: PREDICT
            prediction = await self.predictor.evaluate_edge(target['title'], target['price']/100.0, brief)
            logger.info(f"Model Edge: {prediction['edge']:.4f}")
            
            if prediction['signal'] == "TRADE":
                # STEP 4: RISK & EXECUTE
                allowed, msg, size = self.risk_manager.validate(
                    p_model=prediction['p_model'],
                    p_market=prediction['p_market'],
                    bankroll=self.bankroll,
                    current_daily_loss_pct=self.daily_loss,
                    current_drawdown_pct=self.current_drawdown,
                    concurrent_positions=self.concurrent_positions,
                    daily_api_spend=self.daily_api_spend
                )
            
                if allowed:
                    logger.info(f"TRADE APPROVED! Executing Limit Order for ${size:.2f}")
                    
                    try:
                        # In a real environment, you must handle size scaling per exchange rules.
                        if target['platform'] == 'polymarket':
                            logger.warning(f"LIVE EXECUTION TRIGGERED: Buying {size} on Polymarket for {target['id']}")
                        elif target['platform'] == 'kalshi':
                            logger.warning(f"LIVE EXECUTION TRIGGERED: Buying {size} on Kalshi for {target['id']}")
                            
                        self.concurrent_positions += 1
                        
                        # Log the trade to DB
                        self.trade_logger.log_trade(
                            market_id=target['id'],
                            market_title=target['title'],
                            platform=target['platform'],
                            action="BUY",
                            price=prediction['p_market'],
                            size=size,
                            model_edge=prediction['edge'],
                            research_brief=brief
                        )
                    except Exception as e:
                        logger.error(f"Execution Failed: {e}")
                        
                else:
                    logger.warning(f"Trade rejected by Risk Manager: {msg}")
            else:
                logger.info("Signal is WAIT. Edge is insufficient.")
                
            # Polite sleep to prevent LLM rate limiting (HTTP 429)
            time.sleep(3.0)
            
        logger.info("============== PIPELINE COMPLETE ==============")

    async def run_forever(self):
        logger.info("Starting Polymaster Continuous Worker Daemon")
        while True:
            try:
                await self.run_pipeline()
            except Exception as e:
                logger.error(f"Pipeline encountered an error: {e}")
            
            # Sleep for 15 minutes before running the pipeline again
            logger.info("Pipeline sweep complete. Sleeping for 15 minutes...")
            await asyncio.sleep(900)

if __name__ == "__main__":
    from dotenv import load_dotenv
    load_dotenv()
    
    bot = TradingBotOrchestrator()
    asyncio.run(bot.run_forever())
