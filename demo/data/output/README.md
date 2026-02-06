# Demo Data Output

This folder contains expected output files for the WebApp pricing functions.

## Structure

```
output/
├── expected/                     # Pre-computed reference outputs
│   ├── black_scholes/           # Black-Scholes option pricing
│   │   ├── atm_call_1y.json     # ATM Call, 1Y expiry
│   │   └── atm_put_1y.json      # ATM Put, 1Y expiry
│   ├── garman_kohlhagen/        # FX option pricing
│   │   └── eurusd_call_1y.json  # EURUSD Call, 1Y expiry
│   └── irs/                     # Interest rate swap pricing
│       └── 5y_payer_swap.json   # 5Y Payer IRS
└── README.md
```

## Purpose

These files serve as:
1. **Reference values** for verifying pricing calculations
2. **Expected outputs** for functions not yet implemented in crates
3. **Test fixtures** for integration testing

## JSON Schema

Each expected output file follows this structure:

```json
{
  "description": "Human-readable description",
  "input": {
    // Input parameters for the calculation
  },
  "output": {
    "price": 0.0,
    "greeks": {
      "delta": 0.0,
      "gamma": 0.0,
      "vega": 0.0,
      "theta": 0.0,
      "rho": 0.0
    }
  },
  "source": "crate::module::function",
  "generated_at": "ISO8601 timestamp",
  "notes": "Additional context"
}
```

## Data Sources

- **Black-Scholes**: `pricer_models::analytical::BlackScholes`
- **Garman-Kohlhagen**: `pricer_models::analytical::GarmanKohlhagen`
- **IRS**: `demo_gui::handlers::irs_price` (simplified), future: `pricer_pricing::generic_pricer::GenericPricer`

## Usage

WebApp handlers can load these files to provide expected outputs when crate
functions are not yet available, ensuring consistent API behaviour during
incremental migration.
