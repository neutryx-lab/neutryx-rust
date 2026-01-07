# Gap Analysis: crate-architecture-redesign

## 讎りｦ・

譛ｬ繝峨く繝･繝｡繝ｳ繝医・縲∬ｦ∽ｻｶ縺ｨ譌｢蟄倥さ繝ｼ繝峨・繝ｼ繧ｹ縺ｮ髢薙・繧ｮ繝｣繝・・繧貞・譫舌＠縲∝ｮ溯｣・姶逡･繧堤ｭ門ｮ壹☆繧九◆繧√・諠・ｱ繧呈署萓帙☆繧九・

**蛻・梵蟇ｾ雎｡:**

- 11莉ｶ縺ｮ隕∽ｻｶ・医い繧ｻ繝・ヨ繧ｯ繝ｩ繧ｹ諡｡蠑ｵ縲√・繝ｫ繝√き繝ｼ繝悶・≡蛻ｩ/繧ｯ繝ｬ繧ｸ繝・ヨ/FX/繧ｨ繧ｭ繧ｾ繝√ャ繧ｯ蟇ｾ蠢懶ｼ・
- 譌｢蟄・繧ｯ繝ｬ繝ｼ繝茨ｼ・ricer_core, pricer_models, pricer_kernel, pricer_xva・・

## 1. 迴ｾ迥ｶ隱ｿ譟ｻ

### 1.1 譌｢蟄倥い繧ｻ繝・ヨ荳隕ｧ

| 繧ｯ繝ｬ繝ｼ繝・| 繝｢繧ｸ繝･繝ｼ繝ｫ | 荳ｻ隕√さ繝ｳ繝昴・繝阪Φ繝・|
|----------|-----------|-------------------|
| pricer_core | market_data/curves | `YieldCurve` trait, `FlatCurve`, `InterpolatedCurve` |
| pricer_core | market_data/surfaces | `VolatilitySurface` trait, `FlatVol`, `InterpolatedVolSurface` |
| pricer_core | types | `Currency` (5騾夊ｲｨ), `Date`, `DayCountConvention` (3遞ｮ) |
| pricer_core | math/solvers | `NewtonRaphson`, `Brent` |
| pricer_core | math/interpolators | Linear, CubicSpline, Monotonic, Bilinear |
| pricer_models | instruments | `Instrument<T>` enum (Vanilla, Forward, Swap) |
| pricer_models | models | `StochasticModel` trait, `GBM`, `SingleState`, `TwoFactorState` |
| pricer_models | analytical | Black-Scholes distributions |
| pricer_kernel | mc | `MonteCarloPricer`, `Workspace`, paths |
| pricer_kernel | path_dependent | Asian, Barrier, Lookback |
| pricer_kernel | greeks | `GreeksConfig`, `GreeksMode`, `GreeksResult<T>` |
| pricer_kernel | checkpoint | Checkpointing strategy |
| pricer_xva | portfolio | `Portfolio`, `Counterparty`, `NettingSet`, `Trade` |
| pricer_xva | xva | `XvaCalculator`, CVA/DVA/FVA |
| pricer_xva | soa | `ExposureSoA`, `TradeSoA` |

### 1.2 繧｢繝ｼ繧ｭ繝・け繝√Ε繝代ち繝ｼ繝ｳ

**遒ｺ遶区ｸ医∩繝代ち繝ｼ繝ｳ:**

- **Enum Dispatch**: `Instrument<T>`, `StochasticModelEnum` - trait objects繧帝∩縺鷹撕逧・ョ繧｣繧ｹ繝代ャ繝・
- **Generic Float**: 蜈ｨ蝙九′ `T: Float` 縺ｧ繧ｸ繧ｧ繝阪Μ繝・け・・D莠呈鋤諤ｧ・・
- **Builder Pattern**: `PortfolioBuilder`, `GreeksConfig::builder()`
- **SoA Layout**: `ExposureSoA`, `TradeSoA` 縺ｧ繝吶け繝医Ν蛹匁怙驕ｩ蛹・
- **Workspace Buffers**: MC險育ｮ励〒蜀榊茜逕ｨ蜿ｯ閭ｽ縺ｪ繝舌ャ繝輔ぃ
- **Error Types**: 繧ｯ繝ｬ繝ｼ繝医＃縺ｨ縺ｮ蟆ら畑繧ｨ繝ｩ繝ｼ蝙・

**萓晏ｭ倬未菫・**

```text
pricer_core 竊・pricer_models 竊・pricer_kernel 竊・pricer_xva
     L1            L2              L3            L4
```

### 1.3 邨ｱ蜷医・繧､繝ｳ繝・

- **YieldCurve trait**: 譁ｰ繧ｫ繝ｼ繝門梛縺ｮ霑ｽ蜉繝昴う繝ｳ繝・
- **StochasticModel trait**: 譁ｰ繝｢繝・Ν霑ｽ蜉縺ｮ繧､繝ｳ繧ｿ繝ｼ繝輔ぉ繝ｼ繧ｹ
- **Instrument enum**: 譁ｰ蝠・刀霑ｽ蜉・・ariant霑ｽ蜉・・
- **Portfolio**: 蜿門ｼ慕匳骭ｲ縺ｮ繧ｨ繝ｳ繝医Μ繝昴う繝ｳ繝・

## 2. 隕∽ｻｶ螳溽樟蜿ｯ閭ｽ諤ｧ蛻・梵

### Requirement 1: 繧｢繧ｻ繝・ヨ繧ｯ繝ｩ繧ｹ蛻･蝠・刀髫主ｱ､

| 謚陦楢ｦ∫ｴ | 迴ｾ迥ｶ | 繧ｮ繝｣繝・・ |
|----------|------|---------|
| Instrument enum | 3 variants (Vanilla, Forward, Swap) | 繧｢繧ｻ繝・ヨ繧ｯ繝ｩ繧ｹ蛻･繧ｵ繝貌num縺ｫ蜀肴ｧ区・縺悟ｿ・ｦ・|
| Instrument trait | 譛ｪ螳溯｣・| price(), greeks(), cashflows()縺ｮ蜈ｱ騾壹ヨ繝ｬ繧､繝域眠隕丈ｽ懈・ |
| equity/ module | instruments/逶ｴ荳九↓flat | 繧ｵ繝悶Δ繧ｸ繝･繝ｼ繝ｫ蛹悶′蠢・ｦ・|
| rates/ module | Missing | 譁ｰ隕丈ｽ懈・ |
| credit/ module | Missing | 譁ｰ隕丈ｽ懈・ |
| fx/ module | Missing | 譁ｰ隕丈ｽ懈・ |
| exotic/ module | Missing | 譁ｰ隕丈ｽ懈・ |
| Schedule | Missing | 驥大茜蝠・刀逕ｨ縺ｫ譁ｰ隕丈ｽ懈・ |

**隍・尅蠎ｦ:** M (3-7譌･) - 譌｢蟄脇num縺ｮ蜀肴ｧ区・ + 譁ｰ繝｢繧ｸ繝･繝ｼ繝ｫ霑ｽ蜉
**繝ｪ繧ｹ繧ｯ:** Medium - 蠕梧婿莠呈鋤諤ｧ縺ｮ邯ｭ謖√′蠢・ｦ・

### Requirement 2: 繝槭Ν繝√き繝ｼ繝門ｸょｴ繝・・繧ｿ蝓ｺ逶､

| 謚陦楢ｦ∫ｴ | 迴ｾ迥ｶ | 繧ｮ繝｣繝・・ |
|----------|------|---------|
| YieldCurve trait | 螳溯｣・ｸ医∩ | 諡｡蠑ｵ蜿ｯ閭ｽ |
| CurveSet | Missing | 蜷榊燕莉倥″繧ｫ繝ｼ繝夜寔蜷医・譁ｰ隕丈ｽ懈・ |
| CreditCurve trait | Missing | 繝上じ繝ｼ繝峨Ξ繝ｼ繝郁ｨ育ｮ励・譁ｰ隕上ヨ繝ｬ繧､繝・|
| HazardRateCurve | Missing | CreditCurve螳溯｣・|
| FxVolatilitySurface | Missing | 繝・Ν繧ｿ繝ｻ貅譛溘げ繝ｪ繝・ラ縺ｮ繧ｵ繝ｼ繝輔ぉ繧ｹ |
| MarketDataError | 螳溯｣・ｸ医∩ | 諡｡蠑ｵ縺ｮ縺ｿ |

**隍・尅蠎ｦ:** M (3-7譌･) - 譁ｰ繝医Ξ繧､繝・+ 隍・焚螳溯｣・
**繝ｪ繧ｹ繧ｯ:** Medium - 譌｢蟄倥き繝ｼ繝悶→縺ｮ謨ｴ蜷域ｧ

### Requirement 3: 遒ｺ邇・Δ繝・Ν諡｡蠑ｵ繝輔Ξ繝ｼ繝繝ｯ繝ｼ繧ｯ

| 謚陦楢ｦ∫ｴ | 迴ｾ迥ｶ | 繧ｮ繝｣繝・・ |
|----------|------|---------|
| StochasticModel trait | 螳溯｣・ｸ医∩ | `num_factors()` 霑ｽ蜉縺ｮ縺ｿ |
| SingleState/TwoFactorState | 螳溯｣・ｸ医∩ | 蜊∝・ |
| GBM | 螳溯｣・ｸ医∩ | 蜊∝・ |
| Hull-White | Missing | 譁ｰ隕丞ｮ溯｣・|
| CIR | Missing | 譁ｰ隕丞ｮ溯｣・|
| Heston | Missing | 譁ｰ隕丞ｮ溯｣・|
| LMM | Missing | 譁ｰ隕丞ｮ溯｣・ｼ郁､・尅・・|
| Correlated models | Missing | Cholesky蛻・ｧ｣縺ｮ螳溯｣・|
| Calibrator trait | Missing | 譁ｰ隕上ヨ繝ｬ繧､繝亥ｮ夂ｾｩ |

**隍・尅蠎ｦ:** L (1-2騾ｱ) - 隍・焚繝｢繝・Ν螳溯｣・+ 繧ｭ繝｣繝ｪ繝悶Ξ繝ｼ繧ｷ繝ｧ繝ｳ
**繝ｪ繧ｹ繧ｯ:** High - LMM縺ｮ螳溯｣・､・尅諤ｧ縲・nzyme莠呈鋤諤ｧ遒ｺ隱・

### Requirement 4: 驥大茜繝・Μ繝舌ユ繧｣繝門ｯｾ蠢・

| 謚陦楢ｦ∫ｴ | 迴ｾ迥ｶ | 繧ｮ繝｣繝・・ |
|----------|------|---------|
| Swap struct | 蝓ｺ譛ｬ螳溯｣・≠繧・| 繝ｬ繧ｰ讒矩縺ｮ諡｡蠑ｵ縺悟ｿ・ｦ・|
| InterestRateSwap | Missing | 蝗ｺ螳・螟牙虚繝ｬ繧ｰ縲∵律謨ｰ險育ｮ・|
| Swaption | Missing | 譁ｰ隕丞ｮ溯｣・|
| Cap/Floor | Missing | 譁ｰ隕丞ｮ溯｣・|
| Schedule | Missing | 謾ｯ謇墓律逕滓・繝ｭ繧ｸ繝・け |
| Black76 | Missing | Swaption隗｣譫占ｧ｣ |
| Bachelier | Missing | Normal model |

**隍・尅蠎ｦ:** L (1-2騾ｱ) - 驥大茜蝠・刀縺ｮ蝓ｺ逶､讒狗ｯ・
**繝ｪ繧ｹ繧ｯ:** High - 繧ｹ繧ｱ繧ｸ繝･繝ｼ繝ｫ逕滓・縺ｮ隍・尅諤ｧ縲√き繝ｼ繝夜∈謚槭Ο繧ｸ繝・け

### Requirement 5: 繧ｯ繝ｬ繧ｸ繝・ヨ繝・Μ繝舌ユ繧｣繝門ｯｾ蠢・

| 謚陦楢ｦ∫ｴ | 迴ｾ迥ｶ | 繧ｮ繝｣繝・・ |
|----------|------|---------|
| CreditParams | pricer_xva縺ｫ蟄伜惠 | pricer_core縺ｫ遘ｻ蜍墓､懆ｨ・|
| CDS struct | Missing | 譁ｰ隕丞ｮ溯｣・|
| HazardRateCurve | Missing | Req 2縺ｨ蜈ｱ騾・|
| Default simulation | Missing | MC諡｡蠑ｵ |
| WWR | Missing | CVA險育ｮ玲僑蠑ｵ |

**隍・尅蠎ｦ:** M (3-7譌･) - CDS螳溯｣・+ 繝上じ繝ｼ繝峨Ξ繝ｼ繝医き繝ｼ繝・
**繝ｪ繧ｹ繧ｯ:** Medium - XVA縺ｨ縺ｮ邨ｱ蜷・

### Requirement 6: 轤ｺ譖ｿ繝・Μ繝舌ユ繧｣繝門ｯｾ蠢・

| 謚陦楢ｦ∫ｴ | 迴ｾ迥ｶ | 繧ｮ繝｣繝・・ |
|----------|------|---------|
| Currency enum | 5騾夊ｲｨ螳溯｣・ｸ医∩ | 諡｡蠑ｵ蜿ｯ閭ｽ |
| CurrencyPair | Missing | 譁ｰ隕乗ｧ矩菴・|
| FxOption | Missing | 譁ｰ隕丞ｮ溯｣・|
| FxForward | Missing | 譁ｰ隕丞ｮ溯｣・|
| Garman-Kohlhagen | Missing | FX繧ｪ繝励す繝ｧ繝ｳ隗｣譫占ｧ｣ |
| FxVolatilitySurface | Missing | Req 2縺ｨ蜈ｱ騾・|

**隍・尅蠎ｦ:** M (3-7譌･) - FX蝠・刀 + GK model
**繝ｪ繧ｹ繧ｯ:** Low - 譏守｢ｺ縺ｪ螳溯｣・ヱ繧ｹ

### Requirement 7: 繝ｬ繧､繝､繝ｼ讒区・縺ｨ繝輔か繝ｫ繝讒矩

| 謚陦楢ｦ∫ｴ | 迴ｾ迥ｶ | 繧ｮ繝｣繝・・ |
|----------|------|---------|
| pricer_kernel 竊・pricer_pricing | 繝ｪ繝阪・繝蠢・ｦ・| Cargo.toml + 蜿ら・譖ｴ譁ｰ |
| pricer_xva 竊・pricer_risk | 繝ｪ繝阪・繝蠢・ｦ・| Cargo.toml + 蜿ら・譖ｴ譁ｰ |
| instruments/ sub-modules | flat讒矩 | equity/, rates/, credit/, fx/, exotic/ |
| models/ sub-modules | flat讒矩 | equity/, rates/, hybrid/ |
| feature flags | 譛ｪ螳溯｣・| Cargo.toml features霑ｽ蜉 |

**隍・尅蠎ｦ:** M (3-7譌･) - 螟ｧ隕乗ｨ｡繝ｪ繝輔ぃ繧ｯ繧ｿ繝ｪ繝ｳ繧ｰ
**繝ｪ繧ｹ繧ｯ:** High - 蜈ｨ繧ｯ繝ｬ繝ｼ繝医↓蠖ｱ髻ｿ縲，I/CD繝・せ繝亥ｿ・・

### Requirement 8: 繧ｭ繝｣繝ｪ繝悶Ξ繝ｼ繧ｷ繝ｧ繝ｳ蝓ｺ逶､

| 謚陦楢ｦ∫ｴ | 迴ｾ迥ｶ | 繧ｮ繝｣繝・・ |
|----------|------|---------|
| Newton-Raphson | 螳溯｣・ｸ医∩ | 蜊∝・ |
| Brent | 螳溯｣・ｸ医∩ | 蜊∝・ |
| Levenberg-Marquardt | Missing | 髱樒ｷ壼ｽ｢譛蟆丈ｺ御ｹ玲ｳ輔・譁ｰ隕丞ｮ溯｣・|
| Calibrator trait | Missing | 譁ｰ隕上ヨ繝ｬ繧､繝亥ｮ夂ｾｩ |
| CalibrationError | Missing | 譁ｰ隕上お繝ｩ繝ｼ蝙・|

**隍・尅蠎ｦ:** M (3-7譌･) - L-M繧ｽ繝ｫ繝舌・ + 繝医Ξ繧､繝・
**繝ｪ繧ｹ繧ｯ:** Medium - 謨ｰ蛟､螳牙ｮ壽ｧ縲∝庶譚滓ｧ

### Requirement 9: 繝ｪ繧ｹ繧ｯ繝輔ぃ繧ｯ繧ｿ繝ｼ邂｡逅・

| 謚陦楢ｦ∫ｴ | 迴ｾ迥ｶ | 繧ｮ繝｣繝・・ |
|----------|------|---------|
| GreeksConfig/Result | 螳溯｣・ｸ医∩ | 諡｡蠑ｵ蜿ｯ閭ｽ |
| RiskFactor trait | Missing | 譁ｰ隕上ヨ繝ｬ繧､繝・|
| GreeksAggregator | Missing | 繝昴・繝医ヵ繧ｩ繝ｪ繧ｪ繝ｬ繝吶Ν髮・ｨ・|
| Scenario engine | Missing | 譁ｰ隕丞ｮ溯｣・|
| Preset scenarios | Missing | 繝代Λ繝ｬ繝ｫ/繝・う繧ｹ繝・繝舌ち繝輔Λ繧､ |

**隍・尅蠎ｦ:** M (3-7譌･) - 繝ｪ繧ｹ繧ｯ蝓ｺ逶､讒狗ｯ・
**繝ｪ繧ｹ繧ｯ:** Medium - 譌｢蟄賂reeks邨ｱ蜷・

### Requirement 10: 繝代ヵ繧ｩ繝ｼ繝槭Φ繧ｹ縺ｨ繝｡繝｢繝ｪ蜉ｹ邇・

| 謚陦楢ｦ∫ｴ | 迴ｾ迥ｶ | 繧ｮ繝｣繝・・ |
|----------|------|---------|
| SoA layout | 螳溯｣・ｸ医∩ (pricer_xva) | 蜊∝・ |
| Rayon parallelization | 螳溯｣・ｸ医∩ | 蜊∝・ |
| Workspace buffers | 螳溯｣・ｸ医∩ (pricer_kernel) | 蜊∝・ |
| Checkpointing | 螳溯｣・ｸ医∩ | 蜊∝・ |
| criterion benchmarks | 螳溯｣・ｸ医∩ | 繧｢繧ｻ繝・ヨ繧ｯ繝ｩ繧ｹ蛻･霑ｽ蜉 |

**隍・尅蠎ｦ:** S (1-3譌･) - 繝吶Φ繝√・繝ｼ繧ｯ霑ｽ蜉縺ｮ縺ｿ
**繝ｪ繧ｹ繧ｯ:** Low - 譌｢蟄倥ヱ繧ｿ繝ｼ繝ｳ驕ｩ逕ｨ

### Requirement 11: 繧ｨ繧ｭ繧ｾ繝√ャ繧ｯ繝・Μ繝舌ユ繧｣繝門ｯｾ蠢・

| 謚陦楢ｦ∫ｴ | 迴ｾ迥ｶ | 繧ｮ繝｣繝・・ |
|----------|------|---------|
| Asian/Barrier/Lookback | pricer_kernel縺ｫ螳溯｣・ｸ医∩ | pricer_models縺ｫ遘ｻ蜍墓､懆ｨ・|
| VarianceSwap | Missing | 譁ｰ隕丞ｮ溯｣・|
| VolatilitySwap | Missing | 譁ｰ隕丞ｮ溯｣・|
| Cliquet | Missing | 譁ｰ隕丞ｮ溯｣・|
| Autocallable | Missing | 譁ｰ隕丞ｮ溯｣・|
| Rainbow | Missing | 繝槭Ν繝√い繧ｻ繝・ヨ蟇ｾ蠢・|
| QuantoOption | Missing | Quanto隱ｿ謨ｴ |
| Bermudan | Missing | Longstaff-Schwartz |

**隍・尅蠎ｦ:** XL (2騾ｱ莉･荳・ - 螟壽焚縺ｮ繧ｨ繧ｭ繧ｾ繝√ャ繧ｯ螳溯｣・
**繝ｪ繧ｹ繧ｯ:** High - 隍・尅縺ｪ繝壹う繧ｪ繝輔｀C邊ｾ蠎ｦ

## 3. 螳溯｣・い繝励Ο繝ｼ繝√が繝励す繝ｧ繝ｳ

### Option A: 譌｢蟄倥さ繝ｳ繝昴・繝阪Φ繝域僑蠑ｵ

**蟇ｾ雎｡隕∽ｻｶ:** Req 2, 3, 8, 9, 10

**謌ｦ逡･:**

- YieldCurve trait繧堤ｶｭ謖√＠縲，urveSet/CreditCurve 繧定ｿｽ蜉
- StochasticModel trait繧堤ｶｭ謖√＠縲∵眠繝｢繝・Ν繧弾num variant縺ｨ縺励※霑ｽ蜉
- math/solvers縺ｫLevenberg-Marquardt繧定ｿｽ蜉
- pricer_xva縺ｮGreeks繧恥ricer_core縺ｫ遘ｻ蜍輔＠縺ｦ繝ｪ繧ｹ繧ｯ繝輔ぃ繧ｯ繧ｿ繝ｼ蝓ｺ逶､蛹・

**繝医Ξ繝ｼ繝峨が繝・**

- 笨・譌｢蟄倥ヱ繧ｿ繝ｼ繝ｳ豢ｻ逕ｨ縲∝ｭｦ鄙偵さ繧ｹ繝井ｽ・
- 笨・蠕梧婿莠呈鋤諤ｧ邯ｭ謖√′螳ｹ譏・
- 笶・繝輔ぃ繧､繝ｫ閧･螟ｧ蛹悶Μ繧ｹ繧ｯ
- 笶・雋ｬ蜍吝｢・阜縺梧尠譏ｧ縺ｫ縺ｪ繧句庄閭ｽ諤ｧ

### Option B: 譁ｰ隕上さ繝ｳ繝昴・繝阪Φ繝井ｽ懈・

**蟇ｾ雎｡隕∽ｻｶ:** Req 1, 4, 5, 6, 7, 11

**謌ｦ逡･:**

- pricer_models/instruments/ 驟堺ｸ九↓繧｢繧ｻ繝・ヨ繧ｯ繝ｩ繧ｹ蛻･繝｢繧ｸ繝･繝ｼ繝ｫ譁ｰ隕丈ｽ懈・
- pricer_models/schedules/ 譁ｰ隕上Δ繧ｸ繝･繝ｼ繝ｫ
- pricer_models/instruments/exotic/ 縺ｫ蜈ｨ繧ｨ繧ｭ繧ｾ繝√ャ繧ｯ蝠・刀
- 繧ｯ繝ｬ繝ｼ繝亥錐繝ｪ繝阪・繝・・ernel竊弾ngine, xva竊池isk・・

**繝医Ξ繝ｼ繝峨が繝・**

- 笨・譏守｢ｺ縺ｪ雋ｬ蜍吝・髮｢
- 笨・迢ｬ遶九＠縺溘ユ繧ｹ繝亥庄閭ｽ諤ｧ
- 笶・繝輔ぃ繧､繝ｫ謨ｰ蠅怜刈
- 笶・繧､繝ｳ繧ｿ繝ｼ繝輔ぉ繝ｼ繧ｹ險ｭ險医・隍・尅諤ｧ

### Option C: 繝上う繝悶Μ繝・ラ繧｢繝励Ο繝ｼ繝・ｼ域耳螂ｨ・・

**謌ｦ逡･:**

1. **Phase 1**: 繧ｯ繝ｬ繝ｼ繝亥錐繝ｪ繝阪・繝縺ｨ繝輔か繝ｫ繝蜀肴ｧ区・・・eq 7・・
2. **Phase 2**: 蟶ょｴ繝・・繧ｿ蝓ｺ逶､諡｡蠑ｵ・・eq 2・・
3. **Phase 3**: 繧｢繧ｻ繝・ヨ繧ｯ繝ｩ繧ｹ蛻･蝠・刀霑ｽ蜉・・eq 1, 4, 5, 6・・
4. **Phase 4**: 繝｢繝・Ν諡｡蠑ｵ縺ｨ繧ｭ繝｣繝ｪ繝悶Ξ繝ｼ繧ｷ繝ｧ繝ｳ・・eq 3, 8・・
5. **Phase 5**: 繝ｪ繧ｹ繧ｯ邂｡逅・→繧ｨ繧ｭ繧ｾ繝√ャ繧ｯ・・eq 9, 11・・
6. **Phase 6**: 繝代ヵ繧ｩ繝ｼ繝槭Φ繧ｹ讀懆ｨｼ・・eq 10・・

**谿ｵ髫守噪遘ｻ陦・**

- 譌｢蟄連PI邯ｭ謖√＠縺ｪ縺後ｉ譁ｰ讒矩繧剃ｸｦ陦梧ｧ狗ｯ・
- Feature flag縺ｧ谿ｵ髫守噪譛牙柑蛹・
- 蜷・ヵ繧ｧ繝ｼ繧ｺ螳御ｺ・ｾ後↓繝・せ繝医・繝吶Φ繝√・繝ｼ繧ｯ

**繝医Ξ繝ｼ繝峨が繝・**

- 笨・繝ｪ繧ｹ繧ｯ蛻・淵
- 笨・谿ｵ髫守噪讀懆ｨｼ蜿ｯ閭ｽ
- 笨・繝ｭ繝ｼ繝ｫ繝舌ャ繧ｯ螳ｹ譏・
- 笶・遘ｻ陦梧悄髢謎ｸｭ縺ｮ隍・尅諤ｧ
- 笶・驥崎､・さ繝ｼ繝我ｸ譎ら噪縺ｫ逋ｺ逕・

## 4. 隍・尅蠎ｦ繝ｻ繝ｪ繧ｹ繧ｯ隧穂ｾ｡繧ｵ繝槭Μ

| 隕∽ｻｶ | 隍・尅蠎ｦ | 繝ｪ繧ｹ繧ｯ | 逅・罰 |
|------|--------|--------|------|
| Req 1 | M | Medium | enum蜀肴ｧ区・縲∝ｾ梧婿莠呈鋤諤ｧ |
| Req 2 | M | Medium | 譁ｰ繝医Ξ繧､繝医∵里蟄倥き繝ｼ繝也ｵｱ蜷・|
| Req 3 | L | High | LMM隍・尅諤ｧ縲・nzyme莠呈鋤諤ｧ |
| Req 4 | L | High | 繧ｹ繧ｱ繧ｸ繝･繝ｼ繝ｫ逕滓・縲√き繝ｼ繝夜∈謚・|
| Req 5 | M | Medium | XVA邨ｱ蜷・|
| Req 6 | M | Low | 譏守｢ｺ縺ｪ螳溯｣・ヱ繧ｹ |
| Req 7 | M | High | 蜈ｨ繧ｯ繝ｬ繝ｼ繝亥ｽｱ髻ｿ |
| Req 8 | M | Medium | 謨ｰ蛟､螳牙ｮ壽ｧ |
| Req 9 | M | Medium | 譌｢蟄賂reeks邨ｱ蜷・|
| Req 10 | S | Low | 譌｢蟄倥ヱ繧ｿ繝ｼ繝ｳ驕ｩ逕ｨ |
| Req 11 | XL | High | 隍・尅縺ｪ繝壹う繧ｪ繝輔｀C邊ｾ蠎ｦ |

**邱丞粋隧穂ｾ｡:** **L縲弭L** (2-4騾ｱ) - 谿ｵ髫守噪螳溯｣・耳螂ｨ

## 5. 險ｭ險医ヵ繧ｧ繝ｼ繧ｺ縺ｸ縺ｮ謗ｨ螂ｨ莠矩・

### 蜆ｪ蜈亥ｮ溯｣・・ｺ・

1. **Req 7** (繧ｯ繝ｬ繝ｼ繝亥錐繝ｻ讒矩螟画峩) - 莉悶・蜈ｨ隕∽ｻｶ縺ｮ蝓ｺ逶､
2. **Req 2** (繝槭Ν繝√き繝ｼ繝・ - 驥大茜/繧ｯ繝ｬ繧ｸ繝・ヨ蝠・刀縺ｮ蜑肴署譚｡莉ｶ
3. **Req 1** (蝠・刀髫主ｱ､) - 譁ｰ蝠・刀霑ｽ蜉縺ｮ蝓ｺ逶､
4. **Req 4, 5, 6** (繧｢繧ｻ繝・ヨ繧ｯ繝ｩ繧ｹ蛻･蝠・刀) - 荳ｦ陦悟ｮ溯｣・庄閭ｽ
5. **Req 3, 8** (繝｢繝・Ν繝ｻ繧ｭ繝｣繝ｪ繝悶Ξ繝ｼ繧ｷ繝ｧ繝ｳ) - 蝠・刀螳溯｣・ｾ・
6. **Req 9** (繝ｪ繧ｹ繧ｯ繝輔ぃ繧ｯ繧ｿ繝ｼ) - 蝠・刀繝ｻ繝｢繝・Ν螳梧・蠕・
7. **Req 11** (繧ｨ繧ｭ繧ｾ繝√ャ繧ｯ) - 譛蠕後↓霑ｽ蜉
8. **Req 10** (繝代ヵ繧ｩ繝ｼ繝槭Φ繧ｹ) - 蜈ｨ菴馴壹＠縺ｦ邯咏ｶ・

### 隱ｿ譟ｻ蠢・ｦ∽ｺ矩・

| 鬆・岼 | 隱ｿ譟ｻ蜀・ｮｹ | 蜆ｪ蜈亥ｺｦ |
|------|----------|--------|
| LMM螳溯｣・| BGM vs LMM縲・nzyme莠呈鋤諤ｧ | High |
| Longstaff-Schwartz | 蝗槫ｸｰ蝓ｺ蠎暮未謨ｰ縺ｮ驕ｸ謚・| High |
| 繧ｹ繧ｱ繧ｸ繝･繝ｼ繝ｫ逕滓・ | IMM譌･莉倥√き繝ｬ繝ｳ繝繝ｼ邨ｱ蜷・| Medium |
| Wrong-Way Risk | CVA險育ｮ励∈縺ｮ邨ｱ蜷域婿豕・| Medium |
| Variance Swap | 繝ｬ繝励Μ繧ｱ繝ｼ繧ｷ繝ｧ繝ｳ vs MC | Low |

### 豎ｺ螳壻ｺ矩・

險ｭ險医ヵ繧ｧ繝ｼ繧ｺ縺ｧ莉･荳九ｒ豎ｺ螳壹☆繧句ｿ・ｦ√≠繧・

1. **Instrument trait vs enum dispatch 縺ｮ縺ｿ**: 繝医Ξ繧､繝郁ｿｽ蜉縺ｮ蠢・ｦ∵ｧ
2. **LMM螳溯｣・せ繧ｳ繝ｼ繝・*: 1繝輔ぃ繧ｯ繧ｿ繝ｼ邁｡逡･迚・vs 繝輔ΝLMM
3. **繧ｫ繝ｬ繝ｳ繝繝ｼ螟夜Κ萓晏ｭ・*: chrono諡｡蠑ｵ vs 蟆ら畑繝ｩ繧､繝悶Λ繝ｪ
4. **Feature flag邊貞ｺｦ**: 繧｢繧ｻ繝・ヨ繧ｯ繝ｩ繧ｹ蜊倅ｽ・vs 蝠・刀蜊倅ｽ・
