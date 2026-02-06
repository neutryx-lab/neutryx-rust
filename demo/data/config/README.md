# Configuration Files

This directory contains all calculation and application configuration files for Neutryx demo.

## Directory Structure

```
config/
├── README.md                  # This file
├── settings.json              # Master configuration (JSON)
├── settings.toml              # Master configuration (TOML)
├── instruments.json           # Instrument definitions for curve calibration
├── curves.json                # Curve definitions and calibration settings
├── vol_surfaces.json          # Volatility surface definitions (basic)
├── vol_construction.json      # Volatility surface/cube construction settings
├── vol_instruments.json       # Volatility calibration instrument definitions
├── pricing.json               # Analytical pricing config
├── pricing_monte_carlo.json   # Monte Carlo pricing config
├── pricing_tree.json          # Tree-based pricing config
├── risk.json                  # Risk/Greeks config (bump method)
├── risk_aad.json              # Risk/Greeks config (AAD method)
├── risk_first_order.json      # First-order Greeks only
├── gui_defaults.json          # GUI form default values
├── rate_index_mapping.json    # Currency to rate index mapping
├── enums.json                 # Enum values reference
└── scenarios/                 # Scenario configurations
    ├── rate_shock_up.json
    ├── rate_shock_down.json
    ├── vol_shock.json
    ├── stress_2008.json
    └── curve_steepening.json
```

## Configuration Types

### Master Settings (`settings.json` / `settings.toml`)

Complete application configuration including:
- Engine settings (thread pool, memory limits)
- Database configuration
- Service configuration (REST, gRPC)
- Logging settings
- Pricing configuration
- Risk configuration

### Pricing Configuration

| File | Method | Use Case |
|------|--------|----------|
| `pricing.json` | Analytical | Fast, closed-form solutions |
| `pricing_monte_carlo.json` | Monte Carlo | Path-dependent products |
| `pricing_tree.json` | Binomial/Trinomial | American options |

### Risk Configuration

| File | Method | Use Case |
|------|--------|----------|
| `risk.json` | Bump | Standard finite difference |
| `risk_aad.json` | AAD | Fast Greeks (requires LLVM 18) |
| `risk_first_order.json` | Bump | Delta/Vega only (faster) |

### Curve Construction Configuration

#### Instruments (`instruments.json`)

Defines calibration instruments for yield curve construction:

| Field | Description | Example |
|-------|-------------|---------|
| `id` | Unique identifier | `"USD-OIS-5Y"` |
| `currency` | Currency code | `"USD"` |
| `convention` | Market convention ID | `"USD-SOFR-OIS"` |
| `tenor` | Instrument tenor | `"5Y"`, `"3x6"`, `"EVENT"` |
| `rateIndex` | Associated rate index | `"USD-SOFR"` |
| `eventDate` | Event date (Event instruments only) | `"2026-01-28"` |

**Supported Instrument Types:**

| Type | Convention Pattern | Tenor Format | Description |
|------|-------------------|--------------|-------------|
| Deposit | `*-DEPO` | `"O/N"`, `"1M"`, `"3M"` | Money market deposits |
| OIS | `*-OIS` | `"1M"` - `"50Y"` | Overnight index swaps |
| IRS | `*-SWAP` | `"2Y"` - `"30Y"` | Interest rate swaps |
| FRA | `*-FRA` | `"3x6"`, `"6x12"` | Forward rate agreements |
| Event | `*-EVENT` | `"EVENT"` | Central bank meetings, turns |

**Event Instruments:**

Event instruments represent scheduled market events (central bank meetings, year-end turns) that cause rate jumps:

```json
{
  "id": "FED-2026-01",
  "currency": "USD",
  "convention": "USD-SOFR-EVENT",
  "tenor": "EVENT",
  "eventDate": "2026-01-28",
  "rateIndex": "USD-SOFR"
}
```

The market quote value for Event instruments is the expected rate jump in basis points (e.g., `25.0` for +25bp hike, `-25.0` for -25bp cut).

#### Curves (`curves.json`)

Defines yield curve calibration specifications:

| Field | Description | Example |
|-------|-------------|---------|
| `name` | Unique curve identifier | `"USD-SOFR-Discount"` |
| `rateIndex` | Associated rate index | `"USD-SOFR"` |
| `instruments` | Ordered list of instrument IDs | `["USD-Depo-ON", "USD-OIS-1M", ...]` |
| `calibrationMethod` | Calibration algorithm | `"sequential"` |
| `interpolation` | Interpolation method | `"loglinear"` |
| `allowExtrapolation` | Allow extrapolation | `true` |

**Example Curve with Event Instruments:**

```json
{
  "name": "USD-SOFR-WithEvents",
  "description": "USD SOFR curve with FOMC meeting events for jump-aware calibration",
  "rateIndex": "USD-SOFR",
  "instruments": [
    "USD-Depo-ON",
    "FED-2026-01",
    "USD-OIS-1M",
    "FED-2026-02",
    "USD-OIS-3M",
    "USD-OIS-6M",
    "USD-OIS-1Y",
    "USD-OIS-5Y",
    "USD-OIS-10Y",
    "USD-OIS-30Y"
  ],
  "calibrationMethod": "sequential",
  "interpolation": "loglinear"
}
```

Event instruments are interleaved with standard instruments in maturity order. The calibration engine handles rate jumps at event dates.

### Scenario Configurations

Located in `scenarios/` subdirectory:

| File | Description |
|------|-------------|
| `rate_shock_up.json` | Parallel +100bp rate shock |
| `rate_shock_down.json` | Parallel -100bp rate shock |
| `vol_shock.json` | +25% volatility shock |
| `stress_2008.json` | 2008 Financial Crisis scenario |
| `curve_steepening.json` | Yield curve steepening |

## Usage

### Rust (infra_config)

```rust
use infra_config::{PricingConfig, RiskConfig, Settings};

// Load master settings
let settings = Settings::from_json_str(include_str!("../demo/data/config/settings.json"))?;

// Load individual configs
let pricing = PricingConfig::from_json(include_str!("../demo/data/config/pricing.json"))?;
let risk = RiskConfig::from_json(include_str!("../demo/data/config/risk.json"))?;
```

### Rust (Curve Construction)

```rust
use infra_domain::market::{DefinitionRegistry, MarketRateSet};
use pricer_models::builder::construction::{CurveConstructionEngine, ConstructionConfig};

// 1. Load instrument definitions from JSON
let registry = DefinitionRegistry::from_json(
    include_str!("../demo/data/config/instruments.json")
)?;

// 2. Load curve definitions
registry.load_curves_json(include_str!("../demo/data/config/curves.json"))?;

// 3. Load market rates from external source
let market_rates = MarketRateSet::new();
// ... insert rates including event expected jumps (in bp) ...

// 4. Build the curve with reference date for Event instruments
let engine = CurveConstructionEngine::new(
    ConstructionConfig::new(1e-10)
        .with_max_iterations(100)
        .with_reference_date(2026, 2, 3)  // Required for Event instruments
);

let result = engine.build::<f64>(&registry, &market_rates, "USD-SOFR-WithEvents")?;
println!("5Y discount factor: {:.6}", result.curve.discount_factor(5.0)?);
```

### CLI

```bash
# Use specific config file
cargo run -p service_cli -- --config demo/data/config/settings.toml price

# Override with environment variables
NEUTRYX__PRICING__VALUATION_DATE=2026-02-01 cargo run -p service_cli -- price
```

## Validation

All configurations can be validated:

```rust
let config = PricingConfig::from_json(json)?;
config.validate()?;  // Returns ConfigError on failure
```

## Environment Variable Override

Settings can be overridden with environment variables using `NEUTRYX__` prefix:

```bash
NEUTRYX__ENGINE__THREAD_POOL_SIZE=16
NEUTRYX__PRICING__REPORTING_CURRENCY=EUR
NEUTRYX__RISK__GREEKS_METHOD=aad
```
