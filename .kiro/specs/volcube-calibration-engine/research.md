# Research & Design Decisions

## Summary
- **Feature**: volcube-calibration-engine
- **Discovery Scope**: Complex Integration (新規volcubeモジュール + 既存calibrationインフラ再利用)
- **Key Findings**:
  - Breeden-Litzenberger公式は二次微分の数値安定性が重要（smoothing spline + 数値微分）
  - SABR vol cubeはexpiry-tenor毎に独立カリブレーション後、平面補間が標準パターン
  - 既存`CurveResultCache<T>`パターンがvolcubeキャッシュに完全適用可能

---

## Research Log

### Breeden-Litzenberger数値実装
- **Context**: 要件3.1 確率密度関数の実装方法調査
- **Sources Consulted**:
  - [GitHub: Breeden-Litzenberger formula for risk-neutral densities](https://github.com/PavanAnanthSharma/Breeden-Litzenberger-formula-for-risk-neutral-densities)
  - [MATLAB: Estimating Option-Implied Probability Distributions](https://www.mathworks.com/company/technical-articles/estimating-option-implied-probability-distributions-for-asset-pricing.html)
  - [Medium: Options' Implied Probability - Risk-Neutral Densities](https://antonismolski.medium.com/options-implied-probability-a-dive-into-risk-neutral-densities-4bef5280842f)
  - [Imperial College: Breeden-Litzenberger formula notes](https://www.ma.imperial.ac.uk/~bin06/ULL-Int/ullint6a.pdf)
- **Findings**:
  - **公式**: `f(K) = e^(rT) × d²C/dK²` （コール価格の二次微分）
  - **数値実装**: 中心差分 `[C(K+ΔK) - 2C(K) + C(K-ΔK)] / ΔK²`
  - **数値安定性課題**: 二次微分は数値誤差を増幅するため、IV→smooth spline補間→call価格変換→微分の順序が推奨
  - **smoothing戦略**: 4次smoothing splineがIV補間に効果的、GPD tail extension for extreme strikes
- **Implications**:
  - pricer_coreの`CubicSplineInterpolator`を拡張し、smoothing spline追加を検討
  - `VolCube::probability_density()`は`get_vol`→Black-Scholes call価格→数値微分のパイプライン
  - ΔK選択は数値安定性とバイアスのトレードオフ（推奨: strike間隔の0.5-1.0倍）

### SABR Vol Cube カリブレーション戦略
- **Context**: 要件1.1-1.5 3D vol cubeのカリブレーション方法
- **Sources Consulted**:
  - [Interest Rate: SABR and Mean Reversion Calibration](https://interestrate.pubpub.org/pub/hcabnfsy/release/1)
  - [Imperial College: SABR Model Calibration Thesis](https://www.imperial.ac.uk/media/imperial-college/faculty-of-natural-sciences/department-of-mathematics/math-finance/Cheng_Luo-thesis.pdf)
  - [MATLAB: Price Swaption Using SABR Model](https://www.mathworks.com/help/fininst/pricing-a-swaption-using-the-sabr-model.html)
  - [arXiv: Learning the Exact SABR Model (2025)](https://arxiv.org/html/2510.10343v1)
  - [Deriscope: SABR Model Excel Implementation](https://blog.deriscope.com/index.php/en/excel-quantlib-swaption-sabr)
- **Findings**:
  - **Vol Cube構造**: 3軸 (expiry, tenor, strike) で各vertical line (同一expiry-tenor) が同一SABRパラメータ共有
  - **カリブレーション戦略**: expiry-tenor毎に独立してSABR(α, β, ρ, ν)をカリブレーション
  - **パラメータ補間**: カリブレーション後、expiry-tenor平面でパラメータ補間（Bilinear or Bicubic）
  - **Shifted SABR**: negative rate対応にはshifted SABR (shift ≈ 0.8%) を使用、β通常0.5固定
  - **最新研究**: SABR DNN calibration (2025) はHagan近似より豊かなterm structureを学習可能
- **Implications**:
  - 既存`SABRCalibrator`を再利用し、expiry-tenor毎にカリブレーション
  - `VolCubeConfig`にshifted SABRオプション追加
  - パラメータ補間層を追加（`SabrParameterSurface<T>`）

### キャッシュ無効化メカニズム
- **Context**: 要件5.3 市場データ更新時の自動無効化
- **Sources Consulted**: 既存`CurveResultCache<T>`実装解析
- **Findings**:
  - 既存実装は明示的`clear()`のみ、自動無効化なし
  - `CurveKey`はrates_hash + config_hashでキー生成
  - market data timestamp追跡は未実装
- **Implications**:
  - `VolCubeKey`にtimestamp fieldを追加
  - `VolCubeCache::invalidate_if_stale(current_timestamp)`メソッド追加
  - または`VolCubeBuilder`がbuild時にtimestamp比較して自動再カリブレーション

### Arbitrage-free検証
- **Context**: 要件7.3, 9.1 アービトラージ条件違反検出
- **Sources Consulted**: QuantLib文献、学術論文
- **Findings**:
  - **Butterfly spread**: 中間strikeのcall価格 ≤ 両端strikeの加重平均（凸性条件）
  - **Calendar spread**: 長expiry call ≥ 短expiry call（時間価値条件）
  - **Vol cube固有**: tenor軸でも同様の条件適用可能
  - **実用**: 構築時検証 vs lazy検証のトレードオフ（パフォーマンス vs 安全性）
- **Implications**:
  - `ArbitrageFreeValidator`コンポーネントを追加
  - `VolCubeConfig::validate_arbitrage_free: bool`で有効化制御
  - 違反時は`CalibrationError::ArbitrageFreeViolation`を返す

---

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| **A: 既存拡張** | `VolatilitySurface<T>`を3D対応に拡張 | 最小ファイル追加、既存互換 | 2D/3D混在で複雑化 | tenor軸オプショナル化必要 |
| **B: 新規モジュール** | 独立`volcube/`モジュール新設 | 責任分離明確、テスト容易 | ファイル数増加 | 統合ポイント設計必要 |
| **C: ハイブリッド（推奨）** | 新規`volcube/` + 既存パターン再利用 | 両方の利点、段階的実装可能 | やや複雑 | Option C採用 |

**選択**: Option C (ハイブリッド)

**理由**:
1. 既存`SABRCalibrator`, `CalibrationEngine`, `CurveResultCache`パターン再利用でリスク低減
2. 新規`volcube/`モジュールで責任分離、将来拡張容易
3. `VolSurfaceEnum`への`Cube`variant追加で既存統合維持

---

## Design Decisions

### Decision: 3D補間戦略
- **Context**: `get_vol(expiry, tenor, strike)`の補間方法
- **Alternatives Considered**:
  1. 完全3D Trilinear補間 — 全軸同時補間
  2. 層別補間（expiry-tenor Bilinear + strike SABR/SVI）— 各軸異なる補間
  3. パラメータ補間 — SABR(α,ρ,ν)をexpiry-tenor平面で補間後、strike軸計算
- **Selected Approach**: Option 3 (パラメータ補間)
- **Rationale**:
  - SABR/SVIのsmile特性を保持
  - 業界標準（QuantLib, MATLAB）と一致
  - Arbitrage-free条件維持が容易
- **Trade-offs**: パラメータ補間の数値安定性確保が必要
- **Follow-up**: パラメータ補間のsmoothness検証テスト追加

### Decision: Strike軸表現
- **Context**: 要件6.3 Strike軸表現方式
- **Alternatives Considered**:
  1. Absolute strike — 絶対値
  2. Moneyness (K/F) — Forward比
  3. LogMoneyness ln(K/F) — 対数
  4. Delta — オプションdelta
- **Selected Approach**: enum `StrikeAxisType` で全てサポート、internal storage はLogMoneyness
- **Rationale**:
  - LogMoneynessが数値安定性最高
  - SABR公式はLogMoneynessで定義
  - 外部APIは任意表現をサポートし、内部変換
- **Trade-offs**: 変換コスト（最小限、`T::ln()`程度）
- **Follow-up**: Delta変換にはForward価格とIV必要、lazy計算検討

### Decision: キャッシュ無効化トリガー
- **Context**: 要件5.3 市場データ更新時の無効化
- **Alternatives Considered**:
  1. Timestamp比較 — キャッシュキーにtimestamp埋め込み
  2. Explicit invalidation API — `cache.invalidate(instrument_ids)`
  3. Weak reference — Instrumentへのweak ref、drop時無効化
- **Selected Approach**: Option 1 + 2 (Timestamp + Explicit API)
- **Rationale**:
  - Timestampは自動検出（build時にstale判定）
  - Explicit APIは強制クリア用（シナリオ分析等）
  - Weak referenceはRust所有権モデルと相性悪い
- **Trade-offs**: Timestamp取得コスト（`Instant::now()`程度）
- **Follow-up**: timestamp精度（ミリ秒 vs マイクロ秒）決定

---

## Risks & Mitigations

- **3D補間精度** — proptest + 既知SABRパラメータ再現テストで検証
- **数値微分不安定** — smoothing spline適用、ΔK adaptive選択
- **キャッシュメモリ消費** — LRU capacity制限、metrics監視
- **AAD互換性** — 既存`T: Float`パターン一貫適用、num-dual検証

---

## References

- [GitHub: Breeden-Litzenberger formula for risk-neutral densities](https://github.com/PavanAnanthSharma/Breeden-Litzenberger-formula-for-risk-neutral-densities) — 数値実装例
- [MATLAB: Price Swaption Using SABR Model](https://www.mathworks.com/help/fininst/pricing-a-swaption-using-the-sabr-model.html) — SABR swaption実装
- [arXiv: Learning the Exact SABR Model (2025)](https://arxiv.org/html/2510.10343v1) — 最新SABR研究
- [Imperial College: Breeden-Litzenberger formula](https://www.ma.imperial.ac.uk/~bin06/ULL-Int/ullint6a.pdf) — 理論背景

---
_Generated: 2026-01-23_
