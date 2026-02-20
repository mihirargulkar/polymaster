import sqlite3
import asyncio
import aiohttp
import os
import sys
import json
import re
import uuid
import time
import base64
from datetime import datetime
from dataclasses import dataclass
from typing import Optional, Dict, Any, List, Tuple
from dotenv import load_dotenv

load_dotenv() # Load variables from .env

from kalshi_python_async import ApiClient, Configuration
from kalshi_python_async.api.portfolio_api import PortfolioApi
from kalshi_python_async.api.orders_api import OrdersApi
from kalshi_python_async.api.market_api import MarketApi
from kalshi_python_async.models.create_order_request import CreateOrderRequest
from kalshi_python_async.auth import KalshiAuth
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.asymmetric import padding

# --- Custom SDK Fixes ---

class KalshiAuthWithBody(KalshiAuth):
    """Extended KalshiAuth that includes the request body in the signature."""
    def create_auth_headers(self, method: str, url: str, body: str = "") -> dict:
        current_time_milliseconds = int(time.time() * 1000)
        timestamp_str = str(current_time_milliseconds)

        # Extract path from URL - Ensure it includes the base path if needed
        # For production/demo, the path should be everything after the host
        if url.startswith('http'):
            from urllib.parse import urlparse
            parsed = urlparse(url)
            path = parsed.path
        else:
            path = url.split('?')[0]

        # Create message to sign: timestamp + method + path (BODY IS EXCLUDED IN V2)
        msg_string = timestamp_str + method.upper() + path
        
        # Sign the message using RSA-PSS
        message = msg_string.encode('utf-8')
        signature = self.private_key.sign(
            message,
            padding.PSS(
                mgf=padding.MGF1(hashes.SHA256()),
                salt_length=padding.PSS.DIGEST_LENGTH # Required for Kalshi V2
            ),
            hashes.SHA256()
        )
        signature_b64 = base64.b64encode(signature).decode('utf-8')

        return {
            "KALSHI-ACCESS-KEY": self.key_id,
            "KALSHI-ACCESS-SIGNATURE": signature_b64,
            "KALSHI-ACCESS-TIMESTAMP": timestamp_str,
        }

class KalshiApiClient(ApiClient):
    """Custom ApiClient that handles Kalshi authentication properly for POST requests."""
    async def call_api(self, method, url, header_params=None, body=None, post_params=None, _request_timeout=None) -> Any:
        if header_params is None:
            header_params = {}

        # Use our custom auth handler if present
        if hasattr(self, 'kalshi_auth_with_body') and self.kalshi_auth_with_body:
            # We no longer pre-serialize the body as it's not part of the signature string.
            # This avoids double-serialization bugs in the rest.py layer.
            auth_headers = self.kalshi_auth_with_body.create_auth_headers(method, url)
            header_params.update(auth_headers)
            
            # Implementation of retry logic for 429 Rate Limit
            max_retries = 3
            for attempt in range(max_retries):
                try:
                    # Pass the original 'body' (dict or model) to super().call_api
                    # The SDK's rest.py will handle serialization correctly.
                    return await super().call_api(method, url, header_params, body, post_params, _request_timeout)
                except Exception as e:
                    # Check for 429 status code
                    if "429" in str(e) and attempt < max_retries - 1:
                        wait = (attempt + 1) * 2  # Exponential-ish backoff
                        # We use print here as this is low-level SDK override
                        print(f"WARN: Kalshi Rate Limit (429). Retrying in {wait}s... (Attempt {attempt+1}/{max_retries})")
                        await asyncio.sleep(wait)
                        # Refresh headers for the new attempt (new timestamp)
                        auth_headers = self.kalshi_auth_with_body.create_auth_headers(method, url, body_str)
                        header_params.update(auth_headers)
                        continue
                    raise e

        return await super().call_api(method, url, header_params, body, post_params, _request_timeout)

# --- Configuration & Type Definitions ---

@dataclass
class SimulationConfig:
    db_path: str = "whale_alerts.db"
    kalshi_key_id: str = os.getenv("KALSHI_API_KEY_ID", "0e2163c1-094e-40ad-80bd-238e014b2c4e")
    private_key_path: str = os.getenv("KALSHI_PRIVATE_KEY_PATH", "/Users/mihirargulkar/Documents/PROJECTS/polymaster/kalshi-key.pem")
    kalshi_api_base: str = os.getenv("KALSHI_API_HOST", "https://api.elections.kalshi.com/trade-api/v2")
    gamma_api_base: str = os.getenv("GAMMA_API_BASE", "https://gamma-api.polymarket.com")
    
    # Shadow Portfolio
    starting_bankroll: float = float(os.getenv("STARTING_BANKROLL", "67.22"))
    bet_size: float = float(os.getenv("BET_SIZE", "5.0"))
    min_reserve: float = float(os.getenv("MIN_RESERVE", "10.0"))
    
    # Strategy Filters
    max_price: float = float(os.getenv("MAX_PRICE", "0.97"))

class MarketMatcher:
    def __init__(self, kalshi_api: MarketApi, ollama_model: str = "llama3"):
        self.market_api = kalshi_api
        self.ollama_model = ollama_model
        self.logger = lambda msg: print(f"[{datetime.now().strftime('%H:%M:%S')}][MATCHER] {msg}", flush=True)

    async def update_cache(self):
        """Fetch and cache active Kalshi markets to avoid API spam."""
        try:
            # Fetch simple active markets
            resp = await self.market_api.get_markets(limit=2000, status="open")
            self.market_cache = resp.markets
            self.last_cache_update = datetime.now()
            self.logger(f"Cache updated with {len(self.market_cache)} markets.")
        except Exception as e:
            self.logger(f"Cache update failed: {e}")

    async def find_candidates(self, poly_title: str) -> List[Any]:
        """Search cached Kalshi markets for those sharing 2+ keywords."""
        if not hasattr(self, 'market_cache') or not self.market_cache or (datetime.now() - self.last_cache_update).total_seconds() > 3600:
            await self.update_cache()

        keywords = [w.lower() for w in poly_title.split() if len(w) > 3 and w.lower() not in ["price", "will", "market", "outcome", "2024", "2025", "2026"]]
        candidates = []
        
        if hasattr(self, 'market_cache') and self.market_cache:
            for m in self.market_cache:
                m_title_lower = m.title.lower()
                match_count = sum(1 for kw in keywords if kw in m_title_lower)
                if match_count >= 2: # At least 2 keywords must match
                    candidates.append(m)
                elif match_count == 1 and len(keywords) == 1: # Single keyword case
                    candidates.append(m)
                
            # Sort by match quality (simple heuristic)
            candidates.sort(key=lambda m: sum(1 for kw in keywords if kw in m.title.lower()), reverse=True)
        return candidates[:5] # Return top 5

    async def query_ollama(self, prompt: str) -> Optional[Dict]:
        """Send prompt to local Ollama instance."""
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(
                    "http://localhost:11434/api/generate",
                    json={
                        "model": self.ollama_model,
                        "prompt": prompt,
                        "stream": False,
                        "format": "json"
                    },
                    timeout=10
                ) as resp:
                    if resp.status == 200:
                        result = await resp.json()
                        return json.loads(result['response'])
        except Exception as e:
            self.logger(f"Ollama Error: {e}")
        return None

    async def match_market(self, poly_title: str, poly_outcome: str, candidates: List[Any]) -> Optional[Dict]:
        """Use LLM to match Polymarket event to Kalshi market."""
        if not candidates: return None

        params_str = "\n".join([f"{i+1}. Ticker: {m.ticker} | Title: {m.title}" for i, m in enumerate(candidates)])
        
        prompt = f"""
        Task: Match the Polymarket event to the equivalent Kalshi market.
        
        Polymarket Alert: "{poly_title}" (Outcome: {poly_outcome})
        
        Candidate Kalshi Markets:
        {params_str}
        
        Output JSON ONLY:
        {{
            "match": true/false,
            "ticker": "Ticker symbol of the match",
            "side": "yes/no (which side of the Kalshi market maps to the Polymarket outcome)",
            "confidence": 0.0 to 1.0
        }}
        """
        
        response = await self.query_ollama(prompt)
        if response and response.get('match') and response.get('confidence', 0) > 0.8:
            return response
        return None

    def to_usd(self, cents: int) -> str:
        """Helper to format cents into USD string."""
        return f"${cents / 100:,.2f}"

class PnLTracker:
    def __init__(self, config: SimulationConfig):
        self.config = config
        self.kalshi_client = self._init_kalshi_client()
        self.portfolio_api = PortfolioApi(self.kalshi_client)
        self.orders_api = OrdersApi(self.kalshi_client)
        self.market_api = MarketApi(self.kalshi_client)
        self.http_session = None
        # Initialize LLM Matcher
        self.matcher = MarketMatcher(self.market_api, ollama_model="llama3")

    def _init_kalshi_client(self) -> KalshiApiClient:
        """Initialize custom authenticated Kalshi Async Client."""
        try:
            with open(self.config.private_key_path, "r") as f:
                private_key_pem = f.read()
            
            kalshi_config = Configuration(host=self.config.kalshi_api_base)
            kalshi_config.api_key_id = self.config.kalshi_key_id
            
            client = KalshiApiClient(configuration=kalshi_config)
            # Use our custom auth handler to fix body-signing and SDK reference bugs
            client.kalshi_auth_with_body = KalshiAuthWithBody(self.config.kalshi_key_id, private_key_pem)
            
            return client
        except Exception as e:
            self.log(f"CRITICAL: Failed to init Kalshi client: {e}")
            sys.exit(1)

    def log(self, msg: str):
        """Unified logging method."""
        print(f"[{datetime.now().strftime('%Y-%m-%d %H:%M:%S')}] {msg}", flush=True)

    async def get_kalshi_balance(self) -> Optional[Dict[str, Any]]:
        """Fetch real Kalshi portfolio balance using SDK."""
        try:
            resp = await self.portfolio_api.get_balance()
            return {
                "balance": resp.balance,
                "portfolio_value": resp.portfolio_value
            }
        except Exception as e:
            self.log(f"Error fetching Kalshi balance: {e}")
            return None

    async def get_market_outcome(self, platform: str, market_id: str) -> Optional[str]:
        """Fetch settlement outcome from respective platform API (Async)."""
        if not self.http_session:
            self.http_session = aiohttp.ClientSession()

        try:
            if platform.lower() == 'kalshi':
                try:
                    resp = await self.market_api.get_market(market_id)
                    market = resp.market
                    if market.status == 'settled':
                        return market.result.upper()
                except Exception: pass
            
            elif platform.lower() == 'polymarket':
                async with self.http_session.get(f"{self.config.gamma_api_base}/markets/{market_id}", timeout=5) as r:
                    if r.status == 200:
                        data = await r.json()
                        if data.get('closed') is True:
                            if 'tokens' in data:
                                for token in data.get('tokens', []):
                                    if token.get('winner') is True: return token.get('outcome', '').upper()
                            try:
                                outcomes = data.get('outcomes')
                                prices = data.get('outcomePrices')
                                if isinstance(outcomes, str): outcomes = json.loads(outcomes)
                                if isinstance(prices, str): prices = json.loads(prices)
                                if outcomes and prices:
                                    for i, price in enumerate(prices):
                                        if str(price) == "1": return str(outcomes[i]).upper()
                            except Exception: pass
                            return "RESOLVED"
        except Exception: pass
        return None

    async def update_settlements(self, db_path: str):
        """Poll APIs to update status of open alerts (Async)."""
        def get_open_alerts():
            conn = sqlite3.connect(db_path)
            cursor = conn.cursor()
            cursor.execute("""
                SELECT id, platform, market_id 
                FROM alerts 
                WHERE (status = 'OPEN' OR (status = 'SETTLED' AND settled_outcome IS NULL)) 
                AND market_id IS NOT NULL
            """)
            rows = cursor.fetchall()
            conn.close()
            return rows

        open_alerts = await asyncio.to_thread(get_open_alerts)
        if not open_alerts: return

        updates = []
        for alert_id, platform, market_id in open_alerts:
            outcome = await self.get_market_outcome(platform, market_id)
            if outcome:
                updates.append((outcome, alert_id))
        
        if updates:
            def commit_updates():
                conn = sqlite3.connect(db_path)
                cursor = conn.cursor()
                cursor.executemany("UPDATE alerts SET settled_outcome = ?, status = 'SETTLED' WHERE id = ?", updates)
                conn.commit()
                conn.close()
            await asyncio.to_thread(commit_updates)
            self.log(f"Found {len(updates)} new outcomes.")

    def _parse_expiration_date(self, ctx_json: Optional[str], title: Optional[str], alert_dt: datetime) -> Optional[datetime]:
        """Determine expiration date using Metadata first, then Title heuristic."""
        exp_dt = None
        if ctx_json:
            try:
                ctx = json.loads(ctx_json)
                exp_str = ctx.get("expiration_date")
                if exp_str:
                    exp_dt = datetime.fromisoformat(exp_str.replace('Z', '+00:00'))
            except Exception: pass
            
        if not exp_dt and title:
            try:
                title_clean = " ".join(title.split())
                months = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December",
                          "Jan", "Feb", "Mar", "Apr", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
                for month in months:
                    if month.lower() in title_clean.lower():
                        match = re.search(fr"{re.escape(month)}\s+(\d+)", title_clean, re.IGNORECASE)
                        if match:
                            day = int(match.group(1))
                            month_idx = (months.index(month) % 12) + 1
                            exp_dt = datetime(2026, month_idx, day).replace(tzinfo=alert_dt.tzinfo)
                            break
            except Exception: pass
        return exp_dt

    async def place_kalshi_order(self, ticker: str, side: str, count: int, price_cents: int) -> Optional[str]:
        """Place a Limit Order on Kalshi using SDK."""
        try:
            client_oid = str(uuid.uuid4())
            # Call create_order with kwargs as expected by SDK
            resp = await self.orders_api.create_order(
                ticker=ticker,
                action="buy",
                type="limit",
                side="yes" if side.lower() == 'yes' else "no",
                count=count,
                client_order_id=client_oid,
                yes_price=price_cents if side.lower() == 'yes' else None,
                no_price=price_cents if side.lower() == 'no' else None
            )
            order_id = resp.order.order_id
            self.log(f"✅ ORDER PLACED: {side.upper()} {ticker} @ {price_cents}¢ (ID: {order_id})")
            return order_id
        except Exception as e:
            self.log(f"❌ ORDER FAILED: {e}")
            return None

    async def run_simulation(self) -> float:
        """Run the core shadow portfolio simulation logic (Async)."""
        await self.update_settlements(self.config.db_path)
        
        def get_all_data():
            conn = sqlite3.connect(self.config.db_path)
            cursor = conn.cursor()
            cursor.execute("UPDATE alerts SET shadow_bet_amount = 0, shadow_active = 0, pnl_value = 0 WHERE 1=1")
            cursor.execute("""
                SELECT id, platform, market_id, outcome, price, status, settled_outcome, market_context, timestamp, market_title, live_trade_id 
                FROM alerts ORDER BY id ASC
            """)
            rows = cursor.fetchall()
            conn.close()
            return rows

        all_alerts = await asyncio.to_thread(get_all_data)
        current_bankroll = self.config.starting_bankroll
        updates_list = []
        live_trades_params = []
        future_months = ["march", "april", "may", "june", "july", "august", "september", "october", "november", "december", "2026", "2027", "end of year"]
        live_trading_enabled = os.environ.get("LIVE_TRADING_ENABLED", "false").lower() == "true"
        
        for row in all_alerts:
            alert_id, platform, market_id, bet_outcome, price, status, settled_outcome, ctx_json, alert_ts_str, market_title, live_trade_id = row
            should_skip = False
            if not price or price <= 0 or price >= self.config.max_price:
                should_skip = True
            if not should_skip:
                alert_dt = datetime.fromisoformat(alert_ts_str.replace('Z', '+00:00'))
                exp_dt = self._parse_expiration_date(ctx_json, market_title, alert_dt)
                if exp_dt:
                    if exp_dt.tzinfo is None and alert_dt.tzinfo:
                        exp_dt = exp_dt.replace(tzinfo=alert_dt.tzinfo)
                    delta_days = (exp_dt.date() - alert_dt.date()).days
                    if delta_days > 1 or delta_days < 0: should_skip = True
                else:
                    m_lower = (market_title or "").lower()
                    if any(term in m_lower for term in future_months): should_skip = True

            if not should_skip and (current_bankroll - self.config.bet_size) < self.config.min_reserve:
                should_skip = True

            if should_skip:
                updates_list.append((0, 0, 0, alert_id))
                continue

            shadow_bet = self.config.bet_size
            current_bankroll -= shadow_bet
            shares = shadow_bet / price
            shadow_active = 0
            pnl = 0.0
            
            if status == 'SETTLED' and settled_outcome:
                if settled_outcome == "RESOLVED":
                    current_bankroll += shadow_bet
                else:
                    is_win = (bet_outcome and settled_outcome.upper() == bet_outcome.upper())
                    if is_win:
                        payout = shares * 1.00
                        current_bankroll += payout
                        pnl = payout - shadow_bet
                    else: pnl = -shadow_bet
            else:
                shadow_active = 1
                pnl = 0.0
            
            updates_list.append((shadow_bet, shadow_active, pnl, alert_id))
            
            # --- Live Trading & matching Logic ---
            if live_trading_enabled and not live_trade_id:
                now_utc = datetime.now(alert_dt.tzinfo)
                age_mins = (now_utc - alert_dt).total_seconds() / 60.0
                
                if age_mins < 15:
                    target_ticker = None
                    target_side = None
                    
                    if platform.lower() == 'kalshi':
                        target_ticker = market_id
                        target_side = bet_outcome
                    elif platform.lower() == 'polymarket':
                        # Attempt LLM Match
                        candidates = await self.matcher.find_candidates(market_title)
                        match = await self.matcher.match_market(market_title, bet_outcome, candidates)
                        if match and match.get('match'):
                            target_ticker = match.get('ticker')
                            target_side = match.get('side')
                            self.log(f"🤖 LLM MATCHED: {market_title} -> {target_ticker} ({target_side})")

                    if target_ticker:
                        side = target_side.lower() if target_side else 'yes'
                        price_cents = max(1, min(99, int(price * 100)))
                        count = max(1, int(self.config.bet_size / price))
                        live_trades_params.append((target_ticker, side, count, price_cents, alert_id))

        def commit_shadow():
            conn = sqlite3.connect(self.config.db_path)
            cursor = conn.cursor()
            cursor.executemany("UPDATE alerts SET shadow_bet_amount = ?, shadow_active = ?, pnl_value = ? WHERE id = ?", updates_list)
            conn.commit()
            conn.close()
        await asyncio.to_thread(commit_shadow)

        if live_trades_params:
            for market_id, side, count, price_cents, alert_id in live_trades_params:
                self.log(f"🚀 LIVE TRADE SIGNAL: {market_id} ({side.upper()})")
                order_id = await self.place_kalshi_order(market_id, side, count, price_cents)
                if order_id:
                    def save_live_id():
                        conn = sqlite3.connect(self.config.db_path)
                        cursor = conn.cursor()
                        cursor.execute("UPDATE alerts SET live_trade_id = ? WHERE id = ?", (order_id, alert_id))
                        conn.commit()
                        conn.close()
                    await asyncio.to_thread(save_live_id)
        
        return current_bankroll

    async def log_snapshot(self, current_bankroll: float):
        """Record portfolio snapshot (Real + Shadow)."""
        bal_data = await self.get_kalshi_balance()
        real_bal = bal_data.get('balance', 0) if bal_data else 0
        p_val = bal_data.get('portfolio_value', 0) if bal_data else 0
        
        def get_active_cost():
            conn = sqlite3.connect(self.config.db_path)
            cursor = conn.cursor()
            cursor.execute("SELECT sum(shadow_bet_amount) FROM alerts WHERE shadow_active = 1")
            row = cursor.fetchone()
            conn.close()
            return row[0] if row and row[0] else 0.0
            
        active_cost = await asyncio.to_thread(get_active_cost)
        total_shadow_equity = current_bankroll + active_cost
        
        if bal_data:
            final_equity_cents = real_bal + p_val
            self.log(f"Snapshot (Real): ${final_equity_cents/100:.2f} (Cash: ${real_bal/100:.2f} + Pos: ${p_val/100:.2f})")
        else:
            final_equity_cents = int(total_shadow_equity * 100)
            self.log(f"Snapshot (Shadow): ${total_shadow_equity:.2f} (Cash: ${current_bankroll:.2f} + Active: ${active_cost:.2f})")

        def save_snapshot():
            conn = sqlite3.connect(self.config.db_path)
            cursor = conn.cursor()
            cursor.execute('''
                INSERT INTO portfolio_snapshots (timestamp, balance_cents, portfolio_value_cents, total_equity_cents)
                VALUES (?, ?, ?, ?)
            ''', (datetime.now().isoformat(), real_bal, p_val, final_equity_cents))
            conn.commit()
            conn.close()
        await asyncio.to_thread(save_snapshot)

    async def run_forever(self):
        """Main Async Loop (PnL Tracking Only)."""
        self.log("📈 Starting PnL Tracker (Snapshot Service)...")
        while True:
            try:
                # 1. Update Settlements (Resolve Markets)
                await self.update_settlements(self.config.db_path)
                
                # 2. Log Snapshot (Real Balance)
                # We pass 0 as 'final_bankroll' for shadow since we aren't simulating anymore
                await self.log_snapshot(self.config.starting_bankroll)
                
            except Exception as e:
                self.log(f"CRITICAL LOOP ERROR: {e}")
            await asyncio.sleep(30)

async def main():
    config = SimulationConfig()
    tracker = PnLTracker(config)
    await tracker.run_forever()

if __name__ == "__main__":
    try: asyncio.run(main())
    except KeyboardInterrupt: pass
