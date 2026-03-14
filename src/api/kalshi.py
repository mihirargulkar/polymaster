import os
import time
import requests
import base64
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.asymmetric import padding
from cryptography.hazmat.primitives import serialization
from src.utils import logger

class KalshiClient:
    def __init__(self):
        # Demo API as specified in PRD for Week 1
        self.base_url = "https://api.elections.kalshi.com/trade-api/v2"
        self.key_id = os.getenv("KALSHI_API_KEY_ID")
        self.key_path = os.getenv("KALSHI_PRIVATE_KEY_PATH", "kalshi-key.pem")
        self.key_raw = os.getenv("KALSHI_PRIVATE_KEY_RAW")
        
        try:
            if self.key_raw:
                # Direct string injection via Cloud Environment Variable (easier than Secret Files)
                # Replace explicit "\n" strings with actual newlines if configured that way in .env
                raw_key_bytes = self.key_raw.replace("\\n", "\n").encode('utf-8')
                self.private_key = serialization.load_pem_private_key(
                    raw_key_bytes,
                    password=None,
                )
            else:
                # Fallback to local file lookup
                with open(self.key_path, "rb") as key_file:
                    self.private_key = serialization.load_pem_private_key(
                        key_file.read(),
                        password=None,
                    )
        except Exception as e:
            logger.error(f"Failed to load Kalshi RSA key: {e}")
            self.private_key = None
            
    def _generate_signature(self, method, path):
        if not self.private_key or not self.key_id:
            return {}
            
        timestamp = str(int(time.time() * 1000))
        msg_string = timestamp + method + path
        msg_bytes = msg_string.encode('utf-8')
        
        signature = self.private_key.sign(
            msg_bytes,
            padding.PKCS1v15(),
            hashes.SHA256()
        )
        signature_b64 = base64.b64encode(signature).decode('utf-8')
        
        return {
            "KALSHI-ACCESS-KEY": self.key_id,
            "KALSHI-ACCESS-SIGNATURE": signature_b64,
            "KALSHI-ACCESS-TIMESTAMP": timestamp
        }

    def get_markets(self, limit=100):
        """Fetch active markets from Kalshi."""
        path = f"/markets?limit={limit}"
        headers = self._generate_signature("GET", path)
        try:
            resp = requests.get(f"{self.base_url}{path}", headers=headers)
            resp.raise_for_status()
            return resp.json().get("markets", [])
        except Exception as e:
            logger.error(f"Error fetching Kalshi markets: {e}")
            if 'resp' in locals():
                logger.error(resp.text)
            return []
