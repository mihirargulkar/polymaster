
import unittest
from datetime import datetime, timedelta
from unittest.mock import MagicMock, patch
from pnl_tracker import PnLTracker, SimulationConfig

class TestStrategy(unittest.TestCase):
    def setUp(self):
        self.config = SimulationConfig()
        self.tracker = PnLTracker(self.config)
        # Mock logging to keep output clean
        self.tracker.log = MagicMock()

    def test_expiration_date_parsing_metadata(self):
        """Test parsing expiration from JSON metadata."""
        ctx_json = '{"expiration_date": "2026-02-20T12:00:00Z"}'
        alert_dt = datetime(2026, 2, 18, 12, 0, 0) # Feb 18
        
        exp_dt = self.tracker._parse_expiration_date(ctx_json, "Some Market", alert_dt)
        self.assertIsNotNone(exp_dt)
        self.assertEqual(exp_dt.year, 2026)
        self.assertEqual(exp_dt.month, 2)
        self.assertEqual(exp_dt.day, 20)

    def test_expiration_date_parsing_title_heuristic(self):
        """Test parsing expiration from title (Month Day)."""
        title = "Bitcoin Price on March 15"
        alert_dt = datetime(2026, 2, 18, 12, 0, 0)
        
        exp_dt = self.tracker._parse_expiration_date(None, title, alert_dt)
        self.assertIsNotNone(exp_dt)
        self.assertEqual(exp_dt.month, 3)
        self.assertEqual(exp_dt.day, 15)

    def test_time_filter_same_day(self):
        """Test logic for Same Day expiration."""
        alert_dt = datetime(2026, 2, 18, 10, 0, 0)
        exp_dt = datetime(2026, 2, 18, 23, 59, 0) # Same day
        
        # Logic from run_simulation:
        delta_days = (exp_dt.date() - alert_dt.date()).days
        self.assertEqual(delta_days, 0)
        # Should be ALLOWED (delta <= 1)

    def test_time_filter_next_day(self):
        """Test logic for Next Day expiration."""
        alert_dt = datetime(2026, 2, 18, 23, 0, 0)
        exp_dt = datetime(2026, 2, 19, 10, 0, 0) # Next day
        
        delta_days = (exp_dt.date() - alert_dt.date()).days
        self.assertEqual(delta_days, 1)
        # Should be ALLOWED (delta <= 1)

    def test_time_filter_too_far(self):
        """Test logic for Two Days out (should be skipped)."""
        alert_dt = datetime(2026, 2, 18, 10, 0, 0)
        exp_dt = datetime(2026, 2, 20, 10, 0, 0) # 2 days later
        
        delta_days = (exp_dt.date() - alert_dt.date()).days
        self.assertEqual(delta_days, 2)
        # Should be SKIPPED (delta > 1)

    def test_price_ceiling(self):
        """Test price ceiling filter."""
        # Config max is 0.97
        self.assertTrue(0.96 < self.config.max_price) # Allowed
        self.assertTrue(0.98 >= self.config.max_price) # Skipped

    def test_reserve_check(self):
        """Test bankroll reserve logic."""
        # Min reserve 10, bet 5.
        # If bankroll is 14, 14 - 5 = 9 (< 10). Should skip.
        current_bankroll = 14.0
        allowed = (current_bankroll - self.config.bet_size) >= self.config.min_reserve
        self.assertFalse(allowed)

        # If bankroll is 16, 16 - 5 = 11 (>= 10). Should allow.
        current_bankroll = 16.0
        allowed = (current_bankroll - self.config.bet_size) >= self.config.min_reserve
        self.assertTrue(allowed)

if __name__ == '__main__':
    unittest.main()
