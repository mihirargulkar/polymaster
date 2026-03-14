def calculate_kelly(p_model, p_market, bankroll, kelly_fraction=0.25):
    """
    p_model: Your estimated probability of the event occurring (e.g., 0.65).
    p_market: The market's implied probability (e.g., 0.49).
    bankroll: Total available capital.
    kelly_fraction: 0.25 for Quarter-Kelly, 0.5 for Half-Kelly, 1.0 for Full Kelly.
    """
    if bankroll <= 0:
        return 0.0
    if p_model < 0 or p_model > 1:
        return 0.0
    if p_model <= p_market:
        return 0.0  # No edge
    p = p_model
    # Loss probability
    q = 1.0 - p
    
    # Net odds received on a $1 bet.
    # If market says 0.49 ($0.49 to win $1.00), profit is $0.51 on $0.49 risked.
    # b = Net Decimal Odds = Profit / Wager = (1 - p_market) / p_market
    if p_market <= 0 or p_market >= 1:
        return 0.0
        
    b = (1.0 - p_market) / p_market
    
    # f* is the fraction of the bankroll to wager
    f_star = (p * b - q) / b
    
    if f_star <= 0:
        return 0.0
        
    fraction = f_star * kelly_fraction
    position_size = fraction * bankroll
    
    return position_size

if __name__ == "__main__":
    # Test example from PRD
    # prob=0.70, odds 2:1 (win $200 on $100 bet) => p_market=0.333
    bankroll = 10000
    p_model = 0.70
    p_market = 0.3333333
    
    f_k = calculate_kelly(p_model, p_market, bankroll, 1.0)
    q_k = calculate_kelly(p_model, p_market, bankroll, 0.25)
    
    print(f"Full Kelly: ${f_k:.2f} ({f_k/bankroll*100:.1f}%)")
    print(f"Quarter Kelly: ${q_k:.2f} ({q_k/bankroll*100:.1f}%)")
