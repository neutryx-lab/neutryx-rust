# Research & Design Decisions: crate-architecture-redesign

## Summary

- **Feature**: `crate-architecture-redesign`
- **Discovery Scope**: Complex Integration・域里蟄倥す繧ｹ繝・Β縺ｮ螟ｧ隕乗ｨ｡蜀肴ｧ区・・・
- **Key Findings**:
  - Enum Dispatch繝代ち繝ｼ繝ｳ縺ｯ髱咏噪繝・ぅ繧ｹ繝代ャ繝√〒10蛟阪・繝代ヵ繧ｩ繝ｼ繝槭Φ繧ｹ蜷台ｸ翫・nzyme AD莠呈鋤諤ｧ縺ｫ蠢・・
  - SOFR/OIS繝槭Ν繝√き繝ｼ繝悶ヵ繝ｬ繝ｼ繝繝ｯ繝ｼ繧ｯ縺檎樟莉｣縺ｮ驥大茜繝・Μ繝舌ユ繧｣繝冶ｩ穂ｾ｡縺ｮ讓呎ｺ・
  - Hull-White 1F繝｢繝・Ν縺ｯxva險育ｮ励・讌ｭ逡梧ｨ呎ｺ悶［ean-reversion繝代Λ繝｡繝ｼ繧ｿ縺ｮ驕ｩ蛻・↑驕ｸ謚槭′邊ｾ蠎ｦ縺ｫ驥崎ｦ・
  - Longstaff-Schwartz豕輔・3-5蛟九・Laguerre螟夐・ｼ丞渕蠎暮未謨ｰ縺ｧ螳溽畑逧・↑邊ｾ蠎ｦ繧帝＃謌・

## Research Log

### Enum Dispatch vs Trait Objects

- **Context**: Enzyme AD縺ｨ縺ｮ莠呈鋤諤ｧ繧堤ｶｭ謖√＠縺ｪ縺後ｉ螟壽ｧ倥↑驥題檮蝠・刀繝ｻ繝｢繝・Ν繧呈桶縺・､壽・諤ｧ縺ｮ螳溽樟譁ｹ豕・
- **Sources Consulted**:
  - [enum_dispatch crate](https://docs.rs/enum_dispatch/latest/enum_dispatch/)
  - [Rust Dispatch Explained](https://www.somethingsblog.com/2025/04/20/rust-dispatch-explained-when-enums-beat-dyn-trait/)
  - [Rust Polymorphism Guide](https://www.possiblerust.com/guide/enum-or-trait-object)
- **Findings**:
  - Enum dispatch縺ｯtrait objects縺ｨ豈碑ｼ・＠縺ｦ譛螟ｧ10蛟阪・繝代ヵ繧ｩ繝ｼ繝槭Φ繧ｹ蜷台ｸ・
  - 繧ｳ繝ｳ繝代う繝ｩ縺梧怙驕ｩ蛹厄ｼ医う繝ｳ繝ｩ繧､繝ｳ蛹厄ｼ峨ｒ驕ｩ逕ｨ蜿ｯ閭ｽ縲」table繝ｫ繝・け繧｢繝・・荳崎ｦ・
  - 縲靴losed World縲榊燕謠・ 蜈ｨvariant蝙九′繧ｳ繝ｳ繝代う繝ｫ譎ゅ↓譌｢遏･縺ｧ縺ゅｋ蠢・ｦ・
  - 繧ｳ繝ｼ繝芽・蠑ｵ縺ｮ繝ｪ繧ｹ繧ｯ縺ゅｊ・医Δ繝弱Δ繝ｼ繝輔ぅ繧ｼ繝ｼ繧ｷ繝ｧ繝ｳ・・
- **Implications**:
  - 迴ｾ陦後・`Instrument<T>` enum縲～StochasticModelEnum`繝代ち繝ｼ繝ｳ繧堤ｶ咏ｶ・
  - 繧｢繧ｻ繝・ヨ繧ｯ繝ｩ繧ｹ蛻･縺ｫ繧ｵ繝貌num繧貞ｮ夂ｾｩ縺励√ヨ繝・・繝ｬ繝吶Νenum縺ｧ繝・ぅ繧ｹ繝代ャ繝・
  - Enzyme莠呈鋤諤ｧ縺ｮ縺溘ａ縲∝・蝠・刀繝ｻ繝｢繝・Ν縺ｧenum dispatch邯ｭ謖∝ｿ・・

### 繝槭Ν繝√き繝ｼ繝悶ヵ繝ｬ繝ｼ繝繝ｯ繝ｼ繧ｯ・・OFR/OIS・・

- **Context**: 驥大茜繝・Μ繝舌ユ繧｣繝冶ｩ穂ｾ｡縺ｫ蠢・ｦ√↑繝槭Ν繝√き繝ｼ繝門渕逶､縺ｮ險ｭ險・
- **Sources Consulted**:
  - [CME SOFR Derivatives Pricing](https://www.cmegroup.com/articles/2025/price-and-hedging-usd-sofr-interest-swaps-with-sofr-futures.html)
  - [SOFR Discount - ScienceDirect](https://www.sciencedirect.com/science/article/pii/S0304405X24002125)
  - [Quantifi Curve Construction](https://www.quantifisolutions.com/tackling-interest-rate-curve-construction-complexity/)
- **Findings**:
  - LIBOR蟒・ｭ｢蠕後ヾOFR OIS繧ｫ繝ｼ繝悶′USD繝・Μ繝舌ユ繧｣繝悶・繝・ぅ繧ｹ繧ｫ繧ｦ繝ｳ繝域ｨ呎ｺ・
  - 繝・Η繧｢繝ｫ繧ｫ繝ｼ繝悶ョ繧｣繧ｹ繧ｫ繧ｦ繝ｳ繝・ 繝輔か繝ｯ繝ｼ繝峨Ξ繝ｼ繝井ｺ域ｸｬ逕ｨ縺ｨ繝・ぅ繧ｹ繧ｫ繧ｦ繝ｳ繝育畑縺ｧ蛻･繧ｫ繝ｼ繝・
  - SOFR繧ｫ繝ｼ繝匁ｧ狗ｯ峨・隍・尅諤ｧ: 譌･谺｡蟷ｳ蝮・・■蜿顔噪謾ｯ謇輔＞縲∝ｹｾ菴慕噪隍・茜
  - 遏ｭ譛溘・繝・・繧ｸ繝・ヨ繝ｬ繝ｼ繝医・聞譛溘・繧ｹ繝ｯ繝・・繝ｬ繝ｼ繝医〒繝悶・繝医せ繝医Λ繝・・
- **Implications**:
  - `CurveSet`讒矩菴薙〒蜷榊燕莉倥″繧ｫ繝ｼ繝也ｮ｡逅・ｼ・OIS", "SOFR", "TONAR"遲会ｼ・
  - 蜷・膚蜩√〒繝・ぅ繧ｹ繧ｫ繧ｦ繝ｳ繝医き繝ｼ繝悶→繝輔か繝ｯ繝ｼ繝峨き繝ｼ繝悶ｒ蛻・屬謖・ｮ壼庄閭ｽ縺ｫ
  - 繝悶・繝医せ繝医Λ繝・・繧｢繝ｫ繧ｴ繝ｪ繧ｺ繝縺ｮ螳溯｣・ｼ亥ｰ・擂諡｡蠑ｵ・・

### Hull-White 1F繝｢繝・Ν縺ｨ繧ｭ繝｣繝ｪ繝悶Ξ繝ｼ繧ｷ繝ｧ繝ｳ

- **Context**: 驥大茜繝｢繝・Ν縺ｮ螳溯｣・→xva險育ｮ励∈縺ｮ驕ｩ逕ｨ
- **Sources Consulted**:
  - [Hull-White Wikipedia](https://en.wikipedia.org/wiki/Hull%E2%80%93White_model)
  - [S&P Global Hull-White for xVA](https://www.spglobal.com/marketintelligence/en/mi/research-analysis/xva-modeling-squeezing-accuracy-from-the-industry-standard-hul.html)
  - [KTH Calibration Methods](https://people.kth.se/~aaurell/Teaching/SF2975_HT17/calibration-hull-white.pdf)
- **Findings**:
  - Hull-White 1F縺ｯxVA險育ｮ励・讌ｭ逡梧ｨ呎ｺ悶Δ繝・Ν
  - 繝代Λ繝｡繝ｼ繧ｿ: mean-reversion (ﾎｱ)縲《hort rate volatility (ﾏ・縲∃ｸ(蛻晄悄繧､繝ｼ繝ｫ繝峨き繝ｼ繝悶°繧芽ｨ育ｮ・
  - ﾏ・・ATM co-terminal swaption縺ｫ繧ｭ繝｣繝ｪ繝悶Ξ繝ｼ繧ｷ繝ｧ繝ｳ
  - mean-reversion繝代Λ繝｡繝ｼ繧ｿ縺ｯswaption volatility surface縺ｮ蠖｢迥ｶ縺ｫ螟ｧ縺阪￥蠖ｱ髻ｿ
  - xVA繧ｨ繧ｯ繧ｹ繝昴・繧ｸ繝｣繝ｼ險育ｮ励↓縺ｯChevron蠖｢迥ｶ縺ｮswaption驕ｸ謚槭′譛牙柑
- **Implications**:
  - Hull-White 1F繧呈怙蛻昴・驥大茜繝｢繝・Ν縺ｨ縺励※螳溯｣・
  - `Calibrator` trait縺ｧswaption volatility surface縺ｸ縺ｮ繧ｭ繝｣繝ｪ繝悶Ξ繝ｼ繧ｷ繝ｧ繝ｳ
  - mean-reversion縺ｯ險ｭ螳壼庄閭ｽ繝代Λ繝｡繝ｼ繧ｿ縲・・・譎る俣萓晏ｭ湾iece-wise constant

### Longstaff-Schwartz豕包ｼ・ermudan/American Options・・

- **Context**: Bermudan Swaption縺ｮ譌ｩ譛溯｡御ｽｿ蠅・阜謗ｨ螳・
- **Sources Consulted**:
  - [Original Paper](https://people.math.ethz.ch/~hjfurrer/teaching/LongstaffSchwartzAmericanOptionsLeastSquareMonteCarlo.pdf)
  - [Oxford Advanced MC](http://www2.maths.ox.ac.uk/~gilesm/mc/module_6/american.pdf)
  - [CRAN LSMRealOptions](https://cran.r-project.org/web/packages/LSMRealOptions/vignettes/LSMRealOptions.html)
- **Findings**:
  - 譛蟆丈ｺ御ｹ玲ｳ輔〒邯咏ｶ壻ｾ｡蛟､縺ｮ譚｡莉ｶ莉倥″譛溷ｾ・､繧呈耳螳・
  - In-the-money繝代せ縺ｮ縺ｿ繧貞屓蟶ｰ縺ｫ菴ｿ逕ｨ・亥柑邇・髄荳奇ｼ・
  - 蝓ｺ蠎暮未謨ｰ: Laguerre螟夐・ｼ上・-5蛟九〒螳溽畑逧・ｲｾ蠎ｦ
  - 繝舌う繧｢繧ｹ閠・・: 豎ｺ螳夂畑繝代せ縺ｨ隧穂ｾ｡逕ｨ繝代せ繧貞・髮｢謗ｨ螂ｨ
  - 50,000繝代せ遞句ｺｦ縺ｧ蜿取據
- **Implications**:
  - `pricer_pricing/american/lsm.rs`縺ｧLongstaff-Schwartz螳溯｣・
  - 蝓ｺ蠎暮未謨ｰ縺ｯ`BasisFunction` enum縺ｧ驕ｸ謚槫庄閭ｽ・・olynomial, Laguerre, Hermite・・
  - 2繧ｻ繝・ヨ繝代せ譁ｹ蠑上〒繝舌う繧｢繧ｹ菴取ｸ帙が繝励す繝ｧ繝ｳ

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Enum Dispatch邯咏ｶ・| 譌｢蟄倥ヱ繧ｿ繝ｼ繝ｳ邯ｭ謖√√い繧ｻ繝・ヨ繧ｯ繝ｩ繧ｹ蛻･繧ｵ繝貌num | 10x諤ｧ閭ｽ縲・nzyme莠呈鋤縲√さ繝ｳ繝代う繝ｫ譎よ､懆ｨｼ | variant謨ｰ蠅怜刈縺ｧ繧ｳ繝ｳ繝代う繝ｫ譎る俣蠅・| **謗｡逕ｨ** - 譌｢蟄倥ヱ繧ｿ繝ｼ繝ｳ縲、D蠢・郁ｦ∽ｻｶ |
| Trait Objects | `Box<dyn Instrument>`縺ｧ諡｡蠑ｵ諤ｧ | 繧ｪ繝ｼ繝励Φ諡｡蠑ｵ縲√さ繝ｼ繝臥ｰ｡貎・| 10x諤ｧ閭ｽ菴惹ｸ九・nzyme髱樔ｺ呈鋤 | 蜊ｴ荳・- AD莠呈鋤諤ｧ荳榊庄 |
| Hybrid (enum + trait) | 蝓ｺ譛ｬ縺ｯenum縲∵僑蠑ｵ轤ｹ縺ｧtrait | 譟碑ｻ滓ｧ縺ｨ諤ｧ閭ｽ縺ｮ繝舌Λ繝ｳ繧ｹ | 隍・尅諤ｧ蠅怜刈縲∝｢・阜險ｭ險磯屮 | 蟆・擂讀懆ｨ・- 迴ｾ譎らせ縺ｧ縺ｯ荳崎ｦ・|

## Design Decisions

### Decision: 繧ｯ繝ｬ繝ｼ繝亥錐螟画峩・・ernel竊弾ngine, xva竊池isk・・

- **Context**: 蠖ｹ蜑ｲ繝吶・繧ｹ縺ｮ荳雋ｫ縺励◆蜻ｽ蜷崎ｦ丞援縺ｮ遒ｺ遶・
- **Alternatives Considered**:
  1. 迴ｾ迥ｶ邯ｭ謖・ｼ・ernel, xva・俄・螟画峩繧ｳ繧ｹ繝医ぞ繝ｭ縺縺後』va縺縺代′陬ｽ蜩∝錐
  2. 蠖ｹ蜑ｲ繝吶・繧ｹ・・ngine, risk・俄・荳雋ｫ諤ｧ縺ゅｊ
  3. 讖溯・繝吶・繧ｹ・・ompute, analytics・俄・謚ｽ雎｡逧・☆縺弱ｋ
- **Selected Approach**: Option 2 - `pricer_kernel` 竊・`pricer_pricing`縲～pricer_xva` 竊・`pricer_risk`
- **Rationale**: core/models/engine/risk縺ｧ雋ｬ蜍吶′譏守｢ｺ縲』va縺ｯrisk縺ｮ荳驛ｨ讖溯・
- **Trade-offs**: 蜈ｨ蜿ら・譖ｴ譁ｰ蠢・ｦ√∵里蟄倥Θ繝ｼ繧ｶ繝ｼ縺ｸ縺ｮ蠖ｱ髻ｿ
- **Follow-up**: Cargo.toml譖ｴ譁ｰ縲～pub use`縺ｧ繧ｨ繧､繝ｪ繧｢繧ｹ謠蝉ｾ幢ｼ・eprecation隴ｦ蜻贋ｻ倥″・・

### Decision: 繧｢繧ｻ繝・ヨ繧ｯ繝ｩ繧ｹ蛻･繧ｵ繝悶Δ繧ｸ繝･繝ｼ繝ｫ讒区・

- **Context**: 蝠・刀縺ｨ繝｢繝・Ν縺ｮ謨ｴ逅・婿豕・
- **Alternatives Considered**:
  1. Flat讒矩・育樟迥ｶ・俄・繧ｷ繝ｳ繝励Ν縺縺梧爾縺励↓縺上＞
  2. 繧｢繧ｻ繝・ヨ繧ｯ繝ｩ繧ｹ蛻･・・quity/, rates/, credit/遲会ｼ俄・譏守｢ｺ縺ｪ蛻・｡・
  3. 蝠・刀繧ｿ繧､繝怜挨・・ptions/, swaps/, forwards/・俄・繧｢繧ｻ繝・ヨ讓ｪ譁ｭ縺縺梧ｷｷ蝨ｨ
- **Selected Approach**: Option 2 - 繧｢繧ｻ繝・ヨ繧ｯ繝ｩ繧ｹ蛻･繧ｵ繝悶Δ繧ｸ繝･繝ｼ繝ｫ
- **Rationale**: 驥題檮讌ｭ逡後・讓呎ｺ門・鬘槭√メ繝ｼ繝蛻・球縺ｫ驕ｩ蜷・
- **Trade-offs**: 繝輔ぃ繧､繝ｫ謨ｰ蠅怜刈縲∽ｸ驛ｨ蝠・刀縺ｮ蛻・｡槭′譖匁乂・・uanto縺ｯ exotic? fx?・・
- **Follow-up**: Quanto縺ｯexotic驟堺ｸ九：X髢｢騾｣縺ｯfx驟堺ｸ九〒繧ｯ繝ｭ繧ｹ繝ｪ繝輔ぃ繝ｬ繝ｳ繧ｹ

### Decision: Instrument trait霑ｽ蜉

- **Context**: 蜈ｱ騾壹う繝ｳ繧ｿ繝ｼ繝輔ぉ繝ｼ繧ｹ縺ｮ蠢・ｦ∵ｧ・・eq 1.3・・
- **Alternatives Considered**:
  1. Enum methods縺ｮ縺ｿ・育樟迥ｶ・俄・繧ｷ繝ｳ繝励Ν縺縺後・繝ｪ繝｢繝ｼ繝輔ぅ繧ｺ繝髯仙ｮ夂噪
  2. Instrument trait霑ｽ蜉 窶・蜈ｱ騾壼･醍ｴ・ｮ夂ｾｩ縲∝ｰ・擂縺ｮ諡｡蠑ｵ諤ｧ
  3. 隍・焚trait・・riceable, Hedgeable遲会ｼ俄・邏ｰ邊貞ｺｦ縺縺瑚､・尅
- **Selected Approach**: Option 2 - 蜊倅ｸ`Instrument` trait
- **Rationale**: price(), greeks(), cashflows()縺ｮ蜈ｱ騾壼･醍ｴ・‘num縺ｧ縺ｮ螳溯｣・
- **Trade-offs**: trait螳夂ｾｩ縺ｮ霑ｽ蜉菴懈･ｭ
- **Follow-up**: trait螳夂ｾｩ縺ｯpricer_models/instruments/traits.rs縺ｫ驟咲ｽｮ

### Decision: CurveSet縺ｮ險ｭ險・

- **Context**: 繝槭Ν繝√き繝ｼ繝也ｮ｡逅・・螳溯｣・婿豕包ｼ・eq 2・・
- **Alternatives Considered**:
  1. HashMap<String, Box<dyn YieldCurve>>窶・蜍慕噪縺縺窟D髱樔ｺ呈鋤
  2. CurveSet struct with named fields窶・髱咏噪縺縺悟崋螳・
  3. CurveSet<T> with HashMap<CurveName, CurveEnum<T>>窶・蜷榊燕莉倥″ + enum dispatch
- **Selected Approach**: Option 3 - `CurveSet<T: Float>`讒矩菴・+ `CurveName` enum + `CurveEnum<T>`
- **Rationale**: AD莠呈鋤諤ｧ邯ｭ謖√∝錐蜑堺ｻ倥″邂｡逅・・撕逧・ョ繧｣繧ｹ繝代ャ繝・
- **Trade-offs**: 譁ｰ繧ｫ繝ｼ繝冶ｿｽ蜉譎ゅ↓CurveEnum譖ｴ譁ｰ蠢・ｦ・
- **Follow-up**: CurveName enum縺ｯ"OIS", "SOFR", "Forward", "Discount"遲峨ｒ螳夂ｾｩ

### Decision: Feature Flag邊貞ｺｦ

- **Context**: 譚｡莉ｶ莉倥″繧ｳ繝ｳ繝代う繝ｫ縺ｮ邊貞ｺｦ・・eq 7.4・・
- **Alternatives Considered**:
  1. 繧ｯ繝ｬ繝ｼ繝亥腰菴坂・邊励☆縺弱ｋ
  2. 繧｢繧ｻ繝・ヨ繧ｯ繝ｩ繧ｹ蜊倅ｽ搾ｼ・rates", "credit", "fx"・俄・驕ｩ蛻・↑邊貞ｺｦ
  3. 蝠・刀蜊倅ｽ搾ｼ・irs", "cds", "fxoption"・俄・邏ｰ縺九☆縺弱ｋ
- **Selected Approach**: Option 2 - 繧｢繧ｻ繝・ヨ繧ｯ繝ｩ繧ｹ蜊倅ｽ糠eature flag
- **Rationale**: 萓晏ｭ倬未菫らｮ｡逅・′螳ｹ譏薙√さ繝ｳ繝代う繝ｫ譎る俣縺ｮ譛画э縺ｪ蜑頑ｸ帛庄閭ｽ
- **Trade-offs**: 迚ｹ螳壼膚蜩√・縺ｿ髯､螟悶・荳榊庄
- **Follow-up**: default = ["equity"], optional = ["rates", "credit", "fx", "commodity", "exotic"]

## Risks & Mitigations

| 繝ｪ繧ｹ繧ｯ | 蠖ｱ髻ｿ蠎ｦ | 逋ｺ逕溽｢ｺ邇・| 邱ｩ蜥檎ｭ・|
|--------|--------|----------|--------|
| 繧ｯ繝ｬ繝ｼ繝亥錐螟画峩縺ｫ繧医ｋ譌｢蟄倥Θ繝ｼ繧ｶ繝ｼ蠖ｱ髻ｿ | High | Medium | `pub use`繧ｨ繧､繝ｪ繧｢繧ｹ + deprecation隴ｦ蜻翫〒遘ｻ陦梧悄髢捺署萓・|
| LMM螳溯｣・・隍・尅諤ｧ | High | High | Phase 1縺ｧ縺ｯHull-White 1F縺ｮ縺ｿ縲´MM縺ｯ蟆・擂諡｡蠑ｵ |
| Enzyme莠呈鋤諤ｧ縺ｮ遒ｺ隱堺ｸ崎ｶｳ | High | Medium | 蜷・Δ繝・Ν繝ｻ蝠・刀霑ｽ蜉譎ゅ↓enzyme-mode縺ｧ繝・せ繝・|
| 繧ｹ繧ｱ繧ｸ繝･繝ｼ繝ｫ逕滓・縺ｮ繧ｨ繝・ず繧ｱ繝ｼ繧ｹ | Medium | Medium | chrono萓晏ｭ倥！MM譌･莉倥・縺ｿ蛻晄悄螳溯｣・√き繝ｬ繝ｳ繝繝ｼ縺ｯ蟆・擂諡｡蠑ｵ |
| enum variant謨ｰ蠅怜刈縺ｫ繧医ｋ繧ｳ繝ｳ繝代う繝ｫ譎る俣蠅・| Medium | High | feature flag縺ｧ繧｢繧ｻ繝・ヨ繧ｯ繝ｩ繧ｹ蛻･蛻・屬縲∝ｿ・ｦ√↑繧ゅ・縺ｮ縺ｿ譛牙柑蛹・|
| 蠕梧婿莠呈鋤諤ｧ縺ｮ遐ｴ螢・| High | Medium | 荳ｻ隕、PI縺ｯ邯ｭ謖√∝・驛ｨ讒矩縺ｮ縺ｿ螟画峩縲√そ繝槭Φ繝・ぅ繝・け繝舌・繧ｸ繝ｧ繝九Φ繧ｰ |

## References

- [enum_dispatch - Rust](https://docs.rs/enum_dispatch/latest/enum_dispatch/) 窶・Enum dispatch諤ｧ閭ｽ繝吶Φ繝√・繝ｼ繧ｯ
- [CME SOFR Derivatives Pricing](https://www.cmegroup.com/articles/2025/price-and-hedging-usd-sofr-interest-swaps-with-sofr-futures.html) 窶・SOFR繧ｹ繝ｯ繝・・隧穂ｾ｡
- [S&P Global Hull-White for xVA](https://www.spglobal.com/marketintelligence/en/mi/research-analysis/xva-modeling-squeezing-accuracy-from-the-industry-standard-hul.html) 窶・Hull-White xVA驕ｩ逕ｨ
- [Longstaff-Schwartz Original Paper](https://people.math.ethz.ch/~hjfurrer/teaching/LongstaffSchwartzAmericanOptionsLeastSquareMonteCarlo.pdf) 窶・LSM豕輔・蜴溯ｫ匁枚
- [Oxford Advanced MC Methods](http://www2.maths.ox.ac.uk/~gilesm/mc/module_6/american.pdf) 窶・American option MC
