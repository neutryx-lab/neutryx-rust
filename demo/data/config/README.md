# Configuration Files

This directory contains all calculation and application configuration files for Neutryx demo.

## Directory Structure

```
config/
├── README.md                  # This file
├── settings.json              # Master configuration (JSON)
├── settings.toml              # Master configuration (TOML)
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
