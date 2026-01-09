# Security Design Description (Rust Edition)

## TOE Design

### Subsystems

#### L1: PricerCore (Foundation)

* **Description**: Safe abstractions for math, types, and traits.
* **Modules**: `DualNumber`, `DayCount`, `Smoothing`, `Priceable`

#### L2: PricerModels (Business Logic)

* **Description**: Financial instruments and stochastic models.
* **Modules**: `VanillaOption`, `InterestRateSwap`, `BlackScholes`, `HullWhite`, `CIR`, `HestonModel` (Planned), `SABRModel` (Planned)
* **Model Subdirectories**:
  * `models/equity/` - Equity models (GBM, feature-gated)
  * `models/rates/` - Interest rate models: Hull-White, CIR (feature-gated)
  * `models/hybrid/` - Correlated multi-factor models (feature-gated)

#### L3: PricerPricing (Computation)

* **Description**: Unsafe AD bindings and Monte Carlo engine.
* **Modules**: `EnzymeContext`, `MonteCarloEngine`, `PathGenerator`

#### L4: PricerRisk (Application)

* **Description**: Portfolio aggregation and risk metrics.
* **Modules**: `CVAEngine`, `ExposureCalculator`, `NettingSet`
