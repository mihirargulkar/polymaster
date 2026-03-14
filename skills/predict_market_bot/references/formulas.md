# Mathematical Formulas Reference

## 1. Market Edge
```
edge = p_model - p_market
```
*Note: Only trade when edge > 0.04.*

## 2. Expected Value (EV)
```
EV = p * b - (1 - p)
```
*Where `p` is your model probability, and `b` is the decimal odds minus 1.*

## 3. Mispricing Score
```
delta = (p_model - p_market) / standard_deviation
```
*Z-score of your model versus market divergence. Higher is better.*

## 4. Kelly Criterion (Position Sizing)
```
f* = (p * b - q) / b
```
*Where `p` is win probability, `q` is 1 minus `p` (loss probability), and `b` is the net odds (decimal odds - 1).*
*We use Fractional Kelly (multiply `f*` by 0.25) to reduce variance and limit the risk of ruin.*

## 5. Brier Score
```
BS = (1/n) * sum of (predicted - outcome) squared
```
*Calibration tracking. Lower is better. A well-calibrated model tracks below 0.25.*
