import unittest
from kelly_size import calculate_kelly
from validate_risk import RiskValidator

class TestKellySize(unittest.TestCase):
    def test_basic_kelly(self):
        # 70% win prob, market priced at 33.33% (2:1 odds)
        # b = 2.0
        # f* = (0.7 * 2 - 0.3) / 2 = 1.1 / 2 = 0.55 (55% of bankroll)
        bankroll = 10000
        ans = calculate_kelly(0.70, 0.3333333, bankroll, 1.0)
        self.assertAlmostEqual(ans, 5500.0, places=1)

    def test_quarter_kelly(self):
        bankroll = 10000
        ans_full = calculate_kelly(0.70, 0.3333333, bankroll, 1.0)
        ans_quarter = calculate_kelly(0.70, 0.3333333, bankroll, 0.25)
        self.assertAlmostEqual(ans_quarter, ans_full / 4.0, places=1)
        
    def test_no_edge(self):
        # Model prob equals market prob
        self.assertEqual(calculate_kelly(0.50, 0.50, 10000), 0.0)
        # Model prob less than market prob
        self.assertEqual(calculate_kelly(0.40, 0.50, 10000), 0.0)

    def test_invalid_probabilities(self):
        self.assertEqual(calculate_kelly(-0.1, 0.50, 10000), 0.0)
        self.assertEqual(calculate_kelly(1.1, 0.50, 10000), 0.0)
        self.assertEqual(calculate_kelly(0.60, -0.1, 10000), 0.0)
        self.assertEqual(calculate_kelly(0.60, 1.1, 10000), 0.0)

    def test_extreme_bankroll(self):
        self.assertEqual(calculate_kelly(0.60, 0.50, -100), 0.0)
        self.assertEqual(calculate_kelly(0.60, 0.50, 0), 0.0)

class TestRiskValidator(unittest.TestCase):
    def setUp(self):
        self.validator = RiskValidator()

    def test_approval(self):
        # 60% model vs 50% market -> Edge is 0.10 (passes >0.04 check)
        status, msg, sz = self.validator.validate(0.60, 0.50, 10000, 0.0, 0.0, 0, 0.0)
        self.assertTrue(status)
        self.assertEqual(msg, "APPROVED")
        self.assertGreater(sz, 0)
        # Full Kelly f* = (0.6 * 1 - 0.4) / 1 = 0.20
        # Quarter Kelly = 0.05
        # Size = 10000 * 0.05 = 500
        self.assertAlmostEqual(sz, 500.0)

    def test_max_position_cap(self):
        # Highly confident bet
        # Model 90%, Market 50%
        # Full Kelly = (0.9*1 - 0.1)/1 = 0.8
        # Quarter Kelly = 0.20
        # Raw size = 2000
        # However, Validator limits to 5% of bankroll = 500
        status, msg, sz = self.validator.validate(0.90, 0.50, 10000, 0.0, 0.0, 0, 0.0)
        self.assertTrue(status)
        self.assertEqual(sz, 500.0) # 5% cap

    def test_low_edge(self):
        status, msg, sz = self.validator.validate(0.53, 0.50, 10000, 0.0, 0.0, 0, 0.0)
        self.assertFalse(status)
        self.assertIn("Edge", msg)
        self.assertEqual(sz, 0.0)

    def test_drawdown_limit(self):
        status, msg, sz = self.validator.validate(0.90, 0.50, 10000, 0.0, 0.09, 0, 0.0)
        self.assertFalse(status)
        self.assertIn("drawdown", msg)
        self.assertEqual(sz, 0.0)

    def test_daily_loss_limit(self):
        status, msg, sz = self.validator.validate(0.90, 0.50, 10000, 0.16, 0.0, 0, 0.0)
        self.assertFalse(status)
        self.assertIn("Daily loss", msg)
        self.assertEqual(sz, 0.0)

    def test_concurrent_positions(self):
        status, msg, sz = self.validator.validate(0.90, 0.50, 10000, 0.0, 0.0, 15, 0.0)
        self.assertFalse(status)
        self.assertIn("concurrent", msg)
        self.assertEqual(sz, 0.0)

    def test_api_spend_limit(self):
        status, msg, sz = self.validator.validate(0.90, 0.50, 10000, 0.0, 0.0, 0, 51.0)
        self.assertFalse(status)
        self.assertIn("API cost", msg)
        self.assertEqual(sz, 0.0)

if __name__ == '__main__':
    unittest.main()
