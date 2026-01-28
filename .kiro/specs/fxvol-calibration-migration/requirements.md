# Requirements Document

## Introduction

本ドキュメントは、FXボラティリティサーフェスのキャリブレーションロジックを `demo/gui` から適切なクレート（`pricer_core`、`pricer_models`）へ移行するための要件を定義する。

**背景**: 現在、キャリブレーションロジックが `demo/gui/src/web/handlers/fxvol.rs` に直接実装されており、A-I-P-Sアーキテクチャ違反となっている。ビジネスロジックは `pricer_models` クレートに、数学的な計算は `pricer_core` クレートに配置すべきである。

**目的**: `demo_gui` を薄いHTTPハンドラー層として再構築し、既存資産（`infra_master::trade::instrument_def::fx_vol`、`pricer_core::math::formulas::sabr`、`pricer_models::builder::vol`）を活用した適切なクレート配置を実現する。

---

## Requirements

### Requirement 1: Delta-Strike変換関数

**Objective:** As a 量的開発者, I want デルタ値からストライク価格を計算する関数を使用したい, so that FXオプション市場のデルタクォートをストライクベースのクォートに変換できる

#### Acceptance Criteria

1. When delta_to_strike関数が呼び出される, the pricer_core shall デルタ値、スポット価格、国内金利、外国金利、満期、ボラティリティ、デルタタイプからストライク価格を計算する
2. When DeltaType::SpotDeltaが指定される, the pricer_core shall d1 = Φ⁻¹(delta / exp(-foreign_rate × expiry)) を使用してストライクを計算する
3. When DeltaType::ForwardDeltaが指定される, the pricer_core shall d1 = Φ⁻¹(delta) を使用してストライクを計算する
4. When DeltaType::PremiumAdjustedが指定される, the pricer_core shall プレミアム調整項を含むストライク計算を実行する
5. When strike_to_delta関数が呼び出される, the pricer_core shall ストライク価格からデルタ値を計算する（往復変換をサポート）
6. The pricer_core shall infra_master::trade::instrument_def::fx_vol::DeltaType を使用する

---

### Requirement 2: FXフォワードカーブ

**Objective:** As a 量的開発者, I want スポット価格と金利カーブからFXフォワード価格を計算したい, so that ボラティリティサーフェスキャリブレーションに必要なフォワード価格を取得できる

#### Acceptance Criteria

1. The FxCurve trait shall spot()、forward(expiry)、currency_pair() メソッドを提供する
2. When IrpFxCurve::forward(expiry)が呼び出される, the pricer_models shall forward = spot × df_foreign(expiry) / df_domestic(expiry) を計算する
3. The IrpFxCurve shall currency_pair、spot、domestic_curve、foreign_curve フィールドを保持する
4. When domestic_curveまたはforeign_curveが提供される, the IrpFxCurve shall 既存のYieldCurveトレイトを実装する型を受け入れる
5. The pricer_models::market module shall FxCurve trait と IrpFxCurve struct をエクスポートする

---

### Requirement 3: RR/BF変換ロジック

**Objective:** As a 量的開発者, I want リスクリバーサル（RR）とバタフライ（BF）のクォートをストライク/ボラティリティのクォートに変換したい, so that SABRキャリブレーションに適した入力データを準備できる

#### Acceptance Criteria

1. The DeltaVolSlice struct shall expiry、forward、atm_vol、vol_25d_call、vol_25d_put、vol_10d_call（Option）、vol_10d_put（Option）フィールドを保持する
2. When DeltaVolSlice::from_rr_bf()が呼び出される, the pricer_models shall ATM vol、25D RR、25D BFから各デルタ点のボラティリティを計算する
3. When RR/BF変換が実行される, the pricer_models shall vol_25d_call = atm + bf_25d + rr_25d / 2 を計算する
4. When RR/BF変換が実行される, the pricer_models shall vol_25d_put = atm + bf_25d - rr_25d / 2 を計算する
5. When to_strike_vol_quotes()が呼び出される, the pricer_models shall delta_to_strike関数を使用して各デルタ点のストライクを計算する
6. When to_strike_vol_quotes()が完了する, the pricer_models shall VolQuoteのベクターを返す

---

### Requirement 4: SABRキャリブレーション

**Objective:** As a 量的開発者, I want SABRパラメータ（α、ρ、ν）を市場クォートからキャリブレーションしたい, so that FXボラティリティスマイルを正確にモデル化できる

#### Acceptance Criteria

1. When SabrSliceCalibrator::calibrate_slice()が呼び出される, the pricer_models shall Levenberg-Marquardtアルゴリズムを使用してSABRパラメータを最適化する
2. The calibrate_slice method shall 目的関数として残差ベクター [σ_SABR(K₁) - σ_market₁, ...] を使用する
3. When キャリブレーションが開始される, the pricer_models shall 初期推定値としてα_initial = σ_ATM × F^(1-β)、ρ_initial = -0.2、ν_initial = 0.3 を使用する
4. While キャリブレーションが実行される, the pricer_models shall パラメータ境界（α > 0、-1 < ρ < 1、ν > 0）を適用する
5. The pricer_models shall pricer_core::math::formulas::sabr::sabr_implied_vol を使用してSABRインプライドボラティリティを計算する
6. If キャリブレーションが収束しない, then the pricer_models shall CalibrationError を返す

---

### Requirement 5: FxVolBuilder拡張

**Objective:** As a 量的開発者, I want FxVolBuilderがinfra_masterのFxVolInstrumentを直接受け取れるようにしたい, so that 標準化されたインストゥルメント定義からボラティリティサーフェスを構築できる

#### Acceptance Criteria

1. When FxVolBuilder::with_fx_curve()が呼び出される, the FxVolBuilder shall FxCurveをフォワード計算に使用するために内部フィールドに保持する
2. When FxVolBuilder::with_convention()が呼び出される, the FxVolBuilder shall FxVolConventionをdelta-strike変換に使用するために保持する
3. When FxVolBuilder::add_instrument()が呼び出される, the FxVolBuilder shall FxVolInstrumentを受け取り、同じexpiryのインストゥルメントをグループ化する
4. When ATM/RR/BFが揃う, the FxVolBuilder shall DeltaVolSliceを構築し、to_strike_vol_quotesでストライクベースに変換する
5. When FxVolBuilder::add_instruments()が呼び出される, the FxVolBuilder shall &[FxVolInstrument]を受け取り、各インストゥルメントに対してadd_instrumentを呼ぶ
6. The FxVolBuilder shall infra_master::trade::instrument_def::{FxVolInstrument, FxVolConvention, DeltaType} を使用する

---

### Requirement 6: キャリブレーション診断情報

**Objective:** As a 量的開発者, I want キャリブレーション結果に診断情報を追加したい, so that 収束状況やエラーを確認してキャリブレーション品質を評価できる

#### Acceptance Criteria

1. The SliceCalibrationDiagnostics struct shall expiry、residual（最終残差SSE）、iterations、converged フィールドを保持する
2. When calibrate_slice()が完了する, the pricer_models shall SliceCalibrationDiagnosticsを含む結果を返す
3. The FxVolResult shall diagnostics: Vec<SliceCalibrationDiagnostics> または同等の診断情報フィールドを含む
4. When キャリブレーションが収束する, the SliceCalibrationDiagnostics shall converged = true を設定する
5. When キャリブレーションが収束しない, the SliceCalibrationDiagnostics shall converged = false と最終残差を記録する

---

### Requirement 7: demo_gui簡略化

**Objective:** As a 開発者, I want demo_guiのキャリブレーションロジックを削除してpricer_modelsを使用するようにしたい, so that demo_guiが薄いHTTPハンドラー層として機能し、A-I-P-Sアーキテクチャに準拠する

#### Acceptance Criteria

1. When demo_guiがリファクタリングされる, the fxvol handler shall to_delta_volsメソッド、DeltaVols構造体、delta_to_strike関数、フォワード計算のインラインコードを削除する
2. When calibrate_surfaceハンドラがリクエストを受け取る, the fxvol handler shall FxVolInstrumentBuilderを使用してFxVolInstrumentのリストを構築する
3. When calibrate_surfaceハンドラが実行される, the fxvol handler shall IrpFxCurveを構築してFxVolBuilderに渡す
4. When calibrate_surfaceハンドラが実行される, the fxvol handler shall FxVolBuilderを使用してキャリブレーションを実行する
5. The fxvol handler shall pricer_models::builder::{FxVolBuilder, ...} をインポートする
6. The fxvol handler shall pricer_models::market::fx_curve::IrpFxCurve をインポートする
7. The fxvol handler shall infra_master::trade::instrument_def::{FxVolInstrument, FxVolInstrumentBuilder, FxVolConvention, DeltaType} をインポートする

---

## Non-Functional Requirements

### NFR-1: アーキテクチャ準拠

The implementation shall A-I-P-Sアーキテクチャの依存ルールに準拠する（PricerクレートはServiceやAdapterに依存しない）

### NFR-2: 既存資産の活用

The implementation shall 既存の型定義（DeltaType、Delta、FxVolConvention、FxVolInstrument、SabrParams、VolQuote等）を再利用し、重複を避ける

### NFR-3: テストカバレッジ

The implementation shall 各機能に対応するユニットテストを含む（delta-strike往復変換、フォワード計算、RR/BF変換、SABRキャリブレーション収束）

### NFR-4: エラーハンドリング

The implementation shall キャリブレーション失敗時に適切なCalibrationErrorを返し、診断情報を提供する

---

## Dependencies

### 既存資産（活用すべき実装）

| 場所 | 型/関数 | 用途 |
|------|---------|------|
| infra_master::trade::instrument_def::fx_vol | DeltaType, Delta, FxVolConvention, FxVolInstrument, FxVolInstrumentBuilder | FXボラティリティインストゥルメント定義 |
| pricer_core::math::formulas::sabr | SabrImpliedVolParams, sabr_implied_vol(), sabr_atm_vol() | SABRインプライドボラティリティ計算 |
| pricer_models::builder::vol | FxVolBuilder, VolCubeBuilder, SabrParams, VolQuote, SliceCalibrationConfig, SabrSliceCalibrator | ボラティリティサーフェス構築 |

### 実装順序

```
1. Delta-Strike変換関数 (pricer_core)
   ↓
2. FXフォワードカーブ (pricer_models) [並行可能]
   ↓
3. RR/BF変換ロジック (pricer_models) [1に依存]
   ↓
4. SABRキャリブレーション (pricer_models) [並行可能]
   ↓
5. FxVolBuilder拡張 (pricer_models) [1,2,3に依存]
   ↓
6. キャリブレーション診断情報 (pricer_models) [4に依存]
   ↓
7. demo_gui簡略化 (demo) [1-6に依存]
```

推奨実装順序: 1 → 2 → 4 → 3 → 5 → 6 → 7
