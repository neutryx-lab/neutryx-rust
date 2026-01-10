# Gap Analysis: stochastic-models

## 概要

本ドキュメントは、`stochastic-models` 仕様の要件と既存コードベースの差分を分析する。

---

## 1. 現状調査

### 1.1 既存のモデル構造

```text
crates/pricer_models/src/models/
├── stochastic.rs       # StochasticModel トレイト、SingleState、TwoFactorState
├── model_enum.rs       # StochasticModelEnum (GBM, Heston, HullWhite, CIR)
├── gbm.rs              # GBM モデル ✅ 完全実装
├── heston.rs           # Heston モデル ⚠️ 部分実装
├── sabr.rs             # SABR モデル ❌ トレイト未実装
├── equity/             # Equity モデルカテゴリ
├── rates/
│   ├── hull_white.rs   # Hull-White モデル ✅ 完全実装
│   └── cir.rs          # CIR モデル ✅ 完全実装
└── hybrid/             # ハイブリッドモデル
```

### 1.2 StochasticModel トレイト定義

[stochastic.rs](crates/pricer_models/src/models/stochastic.rs) にて定義済み:

```rust
pub trait StochasticModel<T: Float>: Differentiable {
    type State: StochasticState<T>;
    type Params: Clone;

    fn evolve_step(state: Self::State, dt: T, dw: &[T], params: &Self::Params) -> Self::State;
    fn initial_state(params: &Self::Params) -> Self::State;
    fn brownian_dim() -> usize;
    fn model_name() -> &'static str;
    fn num_factors() -> usize;
}
```

### 1.3 StochasticModelEnum 構成

[model_enum.rs](crates/pricer_models/src/models/model_enum.rs) にて定義:

| バリアント | 状態 | Feature Flag |
|-----------|------|--------------|
| GBM | ✅ 実装済 | なし (常時有効) |
| Heston | ✅ 実装済 | なし (常時有効) |
| HullWhite | ✅ 実装済 | `rates` |
| CIR | ✅ 実装済 | `rates` |
| **SABR** | ❌ **未追加** | - |

### 1.4 キャリブレーションインフラ

[pricer_optimiser](crates/pricer_optimiser/src/calibration/) にて:

- `CalibrationEngine`: 汎用キャリブレーションエンジン
- `CalibrationConfig`: 設定 (max_iterations, tolerance)
- `CalibrationResult`: 結果 (parameters, residual, converged)
- Levenberg-Marquardt ソルバー: [solvers/levenberg_marquardt.rs](crates/pricer_optimiser/src/solvers/levenberg_marquardt.rs)

---

## 2. 要件充足分析

### 2.1 Requirement 1: StochasticModel トレイト統合

| 受入基準 | 現状 | ギャップ |
|---------|------|---------|
| Heston が StochasticModel を実装 | ✅ 実装済 ([heston.rs:1044](crates/pricer_models/src/models/heston.rs#L1044)) | なし |
| SABR が StochasticModel を実装 | ❌ 未実装 | **Missing** |
| Hull-White が StochasticModel を実装 | ✅ 実装済 ([hull_white.rs:168](crates/pricer_models/src/models/rates/hull_white.rs#L168)) | なし |
| StochasticModelEnum に全モデル追加 | ⚠️ SABR のみ未追加 | **Missing** |

### 2.2 Requirement 2: Heston モデル実装

| 受入基準 | 現状 | ギャップ |
|---------|------|---------|
| パラメータ (v0, theta, kappa, xi, rho) | ✅ `HestonParams` で定義済 | なし |
| 2次元状態 [spot, variance] | ✅ `TwoFactorState` 使用 | なし |
| Euler-Maruyama 離散化 | ✅ QE スキーム実装済 (より高精度) | なし |
| brownian_dim() = 2 | ✅ 実装済 | なし |
| 分散非負制約 | ✅ reflection/absorption 実装済 | なし |
| ジェネリック T: Float | ✅ 実装済 | なし |
| Feature flag `equity` | ❌ 現在は常時有効 | **Constraint** |

### 2.3 Requirement 3: SABR モデル実装

| 受入基準 | 現状 | ギャップ |
|---------|------|---------|
| パラメータ (alpha, beta, rho, nu) | ✅ `SABRParams` で定義済 | なし |
| 2次元状態 [forward, volatility] | ❌ 状態型未定義 | **Missing** |
| evolve_step 実装 | ❌ 未実装 | **Missing** |
| brownian_dim() = 2 | ❌ 未定義 | **Missing** |
| beta=0 で Normal SABR | ✅ `is_normal()` 判定あり | なし |
| beta=1 で Lognormal SABR | ✅ `is_lognormal()` 判定あり | なし |
| ジェネリック T: Float | ✅ 実装済 | なし |
| Feature flag `rates` | ❌ 現在は `models/` 直下 | **Constraint** |

### 2.4 Requirement 4: Hull-White モデル実装

| 受入基準 | 現状 | ギャップ |
|---------|------|---------|
| パラメータ (a, sigma, r0) | ✅ `HullWhiteParams` で定義済 | なし |
| 1次元状態 [short_rate] | ✅ `SingleState` 使用 | なし |
| Euler-Maruyama 離散化 | ✅ 実装済 | なし |
| brownian_dim() = 1 | ✅ 実装済 | なし |
| 時間依存 θ(t) | ⚠️ 定数近似のみ | **Unknown** |
| ジェネリック T: Float | ✅ 実装済 | なし |
| Feature flag `rates` | ✅ 実装済 | なし |

### 2.5 Requirement 5: AD 互換性要件

| 受入基準 | 現状 | ギャップ |
|---------|------|---------|
| smooth approximations 使用 | ✅ Heston/SABR で使用 | なし |
| evolve_step で動的割当回避 | ✅ GBM/Heston/HW で遵守 | なし |
| Enzyme AD 勾配テスト | ❌ 未実装 | **Research Needed** |
| num-dual 勾配テスト | ❌ 未実装 | **Research Needed** |
| 固定サイズ配列/タプル使用 | ✅ `SingleState`/`TwoFactorState` | なし |

### 2.6 Requirement 6: キャリブレーションインターフェース

| 受入基準 | 現状 | ギャップ |
|---------|------|---------|
| Heston calibrate 関数 | ❌ 未実装 | **Missing** |
| SABR calibrate 関数 | ❌ 未実装 | **Missing** |
| Hull-White calibrate 関数 | ❌ 未実装 | **Missing** |
| CalibrationError 型 | ⚠️ `OptimiserError` あり | 要拡張 |
| pricer_optimiser 統合 | ⚠️ 汎用エンジンあり | 要ブリッジ |

### 2.7 Requirement 7: 単体テストおよび検証

| 受入基準 | 現状 | ギャップ |
|---------|------|---------|
| Heston モーメントテスト | ⚠️ 基本テストあり | 要拡張 |
| SABR Hagan 公式テスト | ✅ `implied_vol` テストあり | なし |
| Hull-White 債券価格テスト | ❌ 未実装 | **Missing** |
| Enzyme vs num-dual 検証 | ❌ 未実装 | **Research Needed** |
| 境界値テスト | ⚠️ 部分的 | 要拡張 |

---

## 3. 実装アプローチオプション

### Option A: 既存コンポーネント拡張

**対象**: SABR の StochasticModel 実装追加、キャリブレーション機能追加

**変更ファイル**:
- `crates/pricer_models/src/models/sabr.rs` — StochasticModel 実装追加
- `crates/pricer_models/src/models/model_enum.rs` — SABR バリアント追加
- `crates/pricer_models/src/models/heston.rs` — calibrate 関数追加
- `crates/pricer_models/src/models/rates/hull_white.rs` — theta(t) 改善、calibrate 追加

**トレードオフ**:
- ✅ 最小限のファイル変更
- ✅ 既存パターン活用
- ❌ 既存ファイルの肥大化リスク

### Option B: 新規コンポーネント作成

**対象**: キャリブレーションを別モジュールに分離

**新規ファイル**:
- `crates/pricer_models/src/calibration/heston.rs`
- `crates/pricer_models/src/calibration/sabr.rs`
- `crates/pricer_models/src/calibration/hull_white.rs`

**トレードオフ**:
- ✅ 関心の分離が明確
- ✅ テスト容易性向上
- ❌ ファイル数増加
- ❌ 依存関係設計が複雑化

### Option C: ハイブリッドアプローチ (推奨)

**戦略**:
1. SABR の StochasticModel 実装は `sabr.rs` 内に追加 (Option A)
2. StochasticModelEnum への SABR 追加は `model_enum.rs` 修正 (Option A)
3. キャリブレーションは `pricer_optimiser` 側で型別関数として実装 (Option B)
4. Hull-White の theta(t) は既存ファイルで改善 (Option A)

**変更ファイル**:
- `sabr.rs` — StochasticModel 実装
- `model_enum.rs` — SABR バリアント追加
- `pricer_optimiser/src/calibration/` — モデル別キャリブレーション

**トレードオフ**:
- ✅ 既存構造を維持しつつ関心分離
- ✅ pricer_optimiser の役割明確化
- ⚠️ 2クレート間の調整が必要

---

## 4. 実装複雑度とリスク

### 努力見積

| コンポーネント | サイズ | 理由 |
|---------------|--------|------|
| SABR StochasticModel 実装 | **M** | evolve_step 離散化、状態管理が必要 |
| StochasticModelEnum 拡張 | **S** | パターンが確立済み |
| Heston calibrate | **M** | Levenberg-Marquardt 統合、目的関数設計 |
| SABR calibrate | **M** | Hagan 公式逆問題、ATM/smile データ処理 |
| Hull-White calibrate | **M** | Swaption 価格計算との統合 |
| Hull-White theta(t) 改善 | **S** | 既存構造の拡張 |
| AD 検証テスト | **L** | Enzyme セットアップ、比較フレームワーク |

**合計見積**: **L** (1-2 weeks equivalent effort)

### リスク評価

| リスク | レベル | 軽減策 |
|-------|--------|--------|
| SABR 離散化スキーム選択 | Medium | Euler-Maruyama から開始、後で改善 |
| Enzyme AD 互換性 | High | num-dual で先に検証、Enzyme は後続 |
| キャリブレーション収束 | Medium | 適切な初期値、境界制約の設定 |
| Feature flag 整合性 | Low | 既存パターンに従う |

---

## 5. 設計フェーズへの推奨事項

### 優先度高

1. **SABR StochasticModel 実装** — 要件の核心、他に依存なし
2. **StochasticModelEnum SABR 追加** — Monte Carlo 統合に必須
3. **基本テスト追加** — TDD アプローチで実装品質確保

### Research Needed

1. **SABR 離散化スキーム** — Euler vs Higher-order schemes for SABR dynamics
2. **Enzyme AD 検証** — pricer_pricing との統合テスト設計
3. **Swaption 価格計算** — Hull-White キャリブレーション用の解析解

### 優先度中

4. **キャリブレーション関数** — pricer_optimiser との統合設計
5. **Hull-White theta(t)** — 時間依存ドリフトの実装

---

## 6. 結論

既存コードベースは確率過程モデルの基盤が整備されており、要件の大部分は既存パターンの拡張で実現可能。主要ギャップは:

1. **SABR の StochasticModel 未実装** — 最大のギャップ
2. **キャリブレーション機能の欠如** — 全3モデル共通
3. **AD 検証テストの欠如** — Enzyme 統合検証

ハイブリッドアプローチ (Option C) を推奨。SABR 実装を優先し、キャリブレーションは pricer_optimiser で一元管理する設計が適切。
