# Volatility Surface Data

This directory contains sample volatility surface data files for the VolCube Calibration UI.

## File Formats

### Swaption VolCube Data (`.json`)

Schema for swaption volatility cube data:

```json
{
  "index": "string",           // Index identifier (e.g., "usd-sofr-swaption")
  "referenceDate": "string",   // ISO 8601 date (e.g., "2026-01-23")
  "dependentCurves": ["string"], // List of dependent curve IDs
  "instruments": [
    {
      "expiry": number,        // Option expiry in years
      "tenor": number,         // Underlying swap tenor in years
      "strike": number,        // Absolute strike rate (e.g., 0.03 for 3%)
      "impliedVol": number,    // Implied volatility (e.g., 0.20 for 20%)
      "forward": number,       // Forward swap rate at reference date
      "weight": number         // Calibration weight (default: 1.0)
    }
  ]
}
```

### FX Volatility Data (`.json`)

Schema for FX volatility surface data using RR/BF quotes:

```json
{
  "currencyPair": "string",    // Currency pair (e.g., "EURUSD")
  "referenceDate": "string",   // ISO 8601 date
  "spot": number,              // Spot FX rate
  "domesticRate": number,      // Domestic interest rate (cont. comp.)
  "foreignRate": number,       // Foreign interest rate (cont. comp.)
  "quotes": [
    {
      "expiry": number,        // Time to expiry in years
      "atmVol": number,        // ATM implied volatility
      "rr25d": number,         // 25-delta Risk Reversal (Call - Put)
      "bf25d": number,         // 25-delta Butterfly (avg wing - ATM)
      "rr10d": number,         // 10-delta Risk Reversal (optional)
      "bf10d": number          // 10-delta Butterfly (optional)
    }
  ]
}
```

## Available Data Files

### Swaption Data

| File | Index | Currency | Description |
|------|-------|----------|-------------|
| `usd-sofr-swaption.json` | `usd-sofr-swaption` | USD | SOFR-linked swaption vol cube |
| `eur-estr-swaption.json` | `eur-estr-swaption` | EUR | ESTR-linked swaption vol cube |

### FX Options Data

| File | Pair | Description |
|------|------|-------------|
| `eurusd.json` | EUR/USD | G10 major pair, typical negative skew |
| `usdjpy.json` | USD/JPY | G10 pair, positive RR (yen weakness bias) |

## Data Conventions

### Swaption Conventions

- **Strike**: Absolute rate (e.g., 0.03 = 3%)
- **Volatility**: Normal vol by default, can be log-normal
- **Expiry × Tenor Grid**: Standard market grid (1Y, 2Y, 5Y, 10Y expiry × 5Y, 10Y tenor)

### FX Conventions

- **Spot Delta**: Premium-excluded (standard G10 convention)
- **ATM**: Delta-neutral straddle (not ATMF)
- **Risk Reversal**: 25D Call vol - 25D Put vol (positive = call skew)
- **Butterfly**: (25D Call + 25D Put) / 2 - ATM (measures smile curvature)

## Calibration Notes

### SABR Calibration

The default SABR parameters:
- β (beta) = 0.5 (CIR backbone)
- Initial α estimated from ATM vol
- ρ estimated from RR (skew direction)
- ν estimated from BF (smile curvature)

### Validation

Data should satisfy:
- All volatilities positive
- All expiries positive
- Forward rates positive
- Weight ≥ 0 (use 0 to exclude from calibration)
