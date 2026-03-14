import sqlite3
import os
import json
from datetime import datetime, timezone
from src.utils import logger

class TradeLogger:
    def __init__(self, db_path="data/trading_history.db"):
        self.db_path = db_path
        os.makedirs(os.path.dirname(self.db_path), exist_ok=True)
        self._init_db()

    def _init_db(self):
        try:
            with sqlite3.connect(self.db_path) as conn:
                cursor = conn.cursor()
                cursor.execute('''
                    CREATE TABLE IF NOT EXISTS trades (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        timestamp TEXT NOT NULL,
                        market_id TEXT NOT NULL,
                        market_title TEXT NOT NULL,
                        platform TEXT NOT NULL,
                        action TEXT NOT NULL,
                        price REAL NOT NULL,
                        size REAL NOT NULL,
                        model_edge REAL NOT NULL,
                        research_brief TEXT
                    )
                ''')
                conn.commit()
        except Exception as e:
            logger.error(f"Failed to initialize trade logger database: {e}")

    def log_trade(self, market_id: str, market_title: str, platform: str, action: str, price: float, size: float, model_edge: float, research_brief: str = ""):
        try:
            with sqlite3.connect(self.db_path) as conn:
                cursor = conn.cursor()
                timestamp = datetime.now(timezone.utc).isoformat()
                cursor.execute('''
                    INSERT INTO trades (timestamp, market_id, market_title, platform, action, price, size, model_edge, research_brief)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                ''', (timestamp, market_id, market_title, platform, action, price, size, model_edge, str(research_brief)))
                conn.commit()
                logger.info(f"Logged {action} trade on {market_id} to history database.")
        except Exception as e:
            logger.error(f"Failed to log trade to history: {e}")
