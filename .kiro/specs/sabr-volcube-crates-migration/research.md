# Research Log: SABR VolCube Crates Migration

## 概要

本ドキュメントは、SABR VolCubeカリブレーションのcrates移行に関する調査結果を記録する。

**調査範囲**: 既存実装の分析、依存関係の確認、設計判断の根拠

---

## 調査ログ

### Topic 1: LevenbergMarquardtSolver API調査

**調査日**: 2026-01-28
**ソース**: `crates/pricer_core/src/math/solvers/levenberg_marquardt.rs`

**発見事項**:
- LMソルバーは**クロージャベースAPI**を採用: `Fn(&[f64]) -> Vec<f64>`
- トレイトベース`LMProblem<T>`は存在しない
- ヤコビアンは内部で有限差分法により自動計算（`compute_jacobian`関数）
- `LMConfig`で制御可能: `tolerance`, `max_iterations`, `initial_lambda`, `lambda_up`, `lambda_down`

**API署名**:
```rust
pub fn solve<F>(&self, residuals: F, initial_params: Vec<f64>) -> Result<LMResult, SolverError>
where F: Fn(&[f64]) -> Vec<f64>
```

**含意**: 要件3の`LMProblem`トレイトは不要。クロージャで直接残差関数を構築する設計に変更。

---

### Topic 2: SABR Implied Volatility公式調査

**調査日**: 2026-01-28
**ソース**: `crates/pricer_core/src/math/formulas/sabr.rs`

**発見事項**:
- Hagan et al. (2002)公式の完全実装が存在
- `SabrImpliedVolParams<T>`構造体: `forward`, `alpha`, `beta`, `nu`, `rho`, `maturity`
- ATM、Normal (β=0)、Lognormal (β=1)、一般ケースをサポート
- 数値安定性のためスムージング関数使用（`smooth_log`, `smooth_pow`）

**API署名**:
```rust
pub fn sabr_implied_vol<T: Float>(
    params: &SabrImpliedVolParams<T>,
    strike: T,
) -> Result<T, SabrImpliedVolError>
```

**含意**: カリブレーションの残差計算でこの関数を直接使用可能。

---

### Topic 3: 既存VolQuote/SliceCalibrationConfig構造

**調査日**: 2026-01-28
**ソース**: `crates/pricer_models/src/builder/vol/mod.rs`

**現在の構造**:
```rust
pub struct VolQuote<T: Float> {
    pub strike: T,
    pub volatility: T,
    pub forward: T,
    // expiry フィールドなし
}

pub struct SliceCalibrationConfig<T: Float> {
    pub fixed_beta: Option<T>,
    pub max_iterations: usize,
    pub tolerance: T,
    pub initial_alpha: T,
    // initial_rho, initial_nu, lm_lambda, bounds フィールドなし
}
```

**ギャップ**:
- `VolQuote`に`expiry`フィールドが不足
- `SliceCalibrationConfig`にLM制御パラメータと境界制約が不足

---

### Topic 4: CalibrationError既存バリアント

**調査日**: 2026-01-28
**ソース**: `crates/pricer_models/src/builder/error.rs`

**既存バリアント**:
- `ConvergenceFailure { iterations, residual }` - 非収束時
- `NumericalInstability { message }` - 数値エラー時
- `InsufficientData { required, provided }` - データ不足時
- `BoundsViolation { param_name, value, lower, upper }` - 境界違反時

**含意**: 新規エラーバリアント追加不要。既存バリアントで要件6を満たせる。

---

### Topic 5: demo/gui既存実装

**調査日**: 2026-01-28
**ソース**: `demo/gui/src/web/handlers/volcube.rs`（gap-analysis.mdより）

**スタンドアロン関数**:
- `calibrate_sabr_simple()` - LM最適化メインロジック
- `optimize_sabr()` - LMアルゴリズム本体
- `sabr_implied_vol()` - Hagan公式（pricer_coreと重複）
- `black_call_price()` - Black公式（価格計算用）

**含意**: これらは削除対象。pricer_modelsの実装で完全に置き換え可能。

---

## アーキテクチャパターン評価

### Option A: クロージャベースLM統合（採用）

**アプローチ**:
- `SabrSliceCalibrator::calibrate_slice()`内でクロージャを構築
- クロージャがquotes、beta、forward、expiryをキャプチャ
- LMソルバーに直接渡す

**利点**:
- 既存APIとの完全な互換性
- 追加の抽象化レイヤー不要
- LMソルバーの内部ヤコビアン計算を活用

**リスク**:
- クロージャ内でのf64変換が必要（LMソルバーがf64固定）

---

## 設計判断

| 判断 | 根拠 |
|------|------|
| クロージャベースAPI採用 | pricer_coreのLMソルバーがトレイトベースではないため |
| CalibrationError既存バリアント使用 | 必要なエラーケースは既にカバー済み |
| VolQuoteにexpiry追加 | SABR公式のmaturityパラメータに必要 |
| SabrBounds構造体新規作成 | パラメータ境界制約の明示的管理 |

---

## リスクと緩和策

| リスク | 発生確率 | 影響度 | 緩和策 |
|--------|---------|--------|--------|
| f64変換による精度低下 | 低 | 低 | カリブレーションは通常f64で十分 |
| クロージャのライフタイム問題 | 低 | 中 | quotesスライス参照で対応可能 |
| demo/gui依存関係循環 | 低 | 高 | A-I-P-Sレイヤー原則を厳守 |

---

_Generated: 2026-01-28_
_Specification: sabr-volcube-crates-migration_
