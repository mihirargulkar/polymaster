from skills.predict_market_bot.scripts.kelly_size import calculate_kelly

class RiskValidator:
    def __init__(self):
        # PRD Defines strict limits:
        self.MIN_EDGE = 0.04
        self.MAX_POS_PCT = 0.05       # 5% max on single position
        self.MAX_CONCURRENT_POS = 15
        self.MAX_DAILY_LOSS_PCT = 0.15
        self.MAX_DRAWDOWN_PCT = 0.08
        self.MAX_API_SPEND_DAY = 50.0

    def validate(self, p_model: float, p_market: float, bankroll: float,
                 current_daily_loss_pct: float, current_drawdown_pct: float,
                 concurrent_positions: int, daily_api_spend: float,
                 category_exposure_pct: float = 0.0) -> tuple[bool, str, float]:
        """
        Runs through all hardcoded risk rules.
        Returns:
            (is_allowed: bool, reason: str, position_size_usd: float)
        """
        # 1. Edge Filter
        edge = p_model - p_market
        if edge < self.MIN_EDGE:
            return False, f"Edge ({edge:.4f}) is below minimum {self.MIN_EDGE}", 0.0
        
        # 2. Daily Loss Filter
        if current_daily_loss_pct >= self.MAX_DAILY_LOSS_PCT:
            return False, f"Daily loss limit reached ({current_daily_loss_pct:.2%})", 0.0
            
        # 3. Absolute Drawdown Filter
        if current_drawdown_pct >= self.MAX_DRAWDOWN_PCT:
            return False, f"Maximum drawdown limit reached ({current_drawdown_pct:.2%})", 0.0
            
        # 4. Concurrency Filter
        if concurrent_positions >= self.MAX_CONCURRENT_POS: # Using existing constant name
            return False, f"Max limit of {self.MAX_CONCURRENT_POS} positions reached", 0.0
            
        # 5. API Spend Filter
        if daily_api_spend >= self.MAX_API_SPEND_DAY: # Using existing constant name
            return False, f"Daily API spend limit reached (${daily_api_spend:.2f})", 0.0
            
        # 6. Concentration Filter (Bulletproof Update)
        if category_exposure_pct >= 0.15:
            return False, f"Max category concentration limit reached (15%)", 0.0
            
        # If we pass all filters, calculate size
        trade_size = calculate_kelly(p_model, p_market, bankroll)
        
        if trade_size <= 0:
            return False, "Kelly calculation yielded <= 0", 0.0

        # 7. Max Single Exposure Cap
        max_allowed_size = bankroll * self.MAX_POS_PCT
        final_size = min(trade_size, max_allowed_size)

        return True, "APPROVED", final_size

if __name__ == "__main__":
    validator = RiskValidator()
    # Mock pass
    res, msg, sz = validator.validate(
        p_model=0.65, 
        p_market=0.49, 
        bankroll=10000,
        current_daily_loss_pct=0.01,
        current_drawdown_pct=0.02,
        concurrent_positions=5,
        daily_api_spend=10.0
    )
    print(f"{msg}: ${sz:.2f}")

    # Mock fail (Low edge)
    res, msg, sz = validator.validate(0.51, 0.49, 10000, 0, 0, 0, 0)
    print(f"{msg}: ${sz:.2f}")
