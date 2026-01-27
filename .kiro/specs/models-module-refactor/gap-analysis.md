# Gap Analysis: models-module-refactor

## 概要

`pricer_models` クレート内の `models/` および `analytical/` モジュールの構造的見直しに関するギャップ分析。

**分析日**: 2026-01-27

---

## 1. 現状調査

### 1.1 ディレクトリ構成

```
crates/pricer_models/src/
├── models/                 # 確率過程モデル
│   ├── mod.rs              # モジュール定義、re-exports
│   ├── stochastic.rs       # StochasticModel trait、状態型
│   ├── model_enum.rs       # StochasticModelEnum (静的ディスパッチ)
│   ├── validation.rs       # パラメータ検証
│   ├── error.rs            # ModelError
│   ├── gbm.rs              # GBM (feature = "equity")
│   ├── heston.rs           # Heston (feature = "equity")
│   ├── sabr.rs             # SABR ★ 複合 (feature = "equity")
│   ├── hull_white.rs       # Hull-White (feature = "rates")
│   ├── cir.rs              # CIR (feature = "rates")
│   └── correlated.rs       # 相関モデル (feature = "exotic")
│
├── analytical/             # 閉形式解
│   ├── mod.rs              # モジュール定義
│   ├── black_scholes.rs    # Black-Scholes
│   ├── bachelier.rs        # Bachelier (Normal model)
│   ├── garman_kohlhagen.rs # Garman-Kohlhagen (FX)
│   ├── distributions.rs    # ★ pricer_coreからのre-export
│   └── error.rs            # AnalyticalError
│
└── market/
    ├── calibration/
    │   └── sabr.rs         # SABR キャリブレーション
    └── volcube/
        └── cube.rs         # VolCube (SABRベース)
```

### 1.2 主要な発見事項

| 項目 | 現状 | 備考 |
|------|------|------|
| `analytical/distributions.rs` | **既に re-export のみ** | `pricer_core::math::distributions` から re-export |
| `models/sabr.rs` | **複合モジュール** | StochasticModel実装 + Hagan公式（約2600行） |
| 外部依存 | 21ファイルが `SABRParams`/`SABRModel` を参照 | VolCube、キャリブレーションが主要消費者 |
| `pricer_core` 分布関数 | 完全実装済み | `norm_cdf`, `norm_pdf`, `norm_inv_cdf` |

### 1.3 SABRモジュール詳細分析

`models/sabr.rs` の内容:

| 機能 | 行番号 | 分類 | 用途 |
|------|--------|------|------|
| `SABRError` | 80-160 | 共通 | エラー型 |
| `SABRParams<T>` | 188-400 | 共通 | パラメータ構造体 |
| `SABRModel<T>` | 421-1000 | **解析** | Hagan公式によるIV計算 |
| `impl StochasticModel` | 1022-1100 | **確率過程** | MC用 evolve_step |
| `impl EquityModel/RatesModel/FxModel` | 1100-1120 | 確率過程 | マーカートレイト |
| テスト | 1200-2700 | 共通 | ユニットテスト |

**重要**: `SABRModel` は両方の機能を持つ複合型。分離には慎重なAPI設計が必要。

---

## 2. 要件実現性分析

### 2.1 要件マッピング

| 要件 | 技術的ニーズ | 現状 | ギャップ |
|------|-------------|------|----------|
| Req 1.1 | models/ に確率過程のみ | SABR が混在 | **要分離** |
| Req 1.2 | analytical/ に閉形式解 | 基本OK | SABR IV追加必要 |
| Req 2.1 | distributions.rs 削除 | 既に re-export | **実質完了** |
| Req 2.2 | pricer_core からの re-export | 実装済み | 削除のみ |
| Req 3.1 | SABRModel (MC用) | 実装済み | 維持 |
| Req 3.2 | SabrImpliedVol (IV用) | 未分離 | **新規作成** |
| Req 4.1 | L1/L2 境界維持 | OK | 確認済み |
| Req 5.1 | 後方互換 API | - | **re-export 必要** |
| Req 6.1 | テスト通過 | 全テスト必要 | リファクタ後検証 |

### 2.2 ギャップ詳細

#### ギャップ 1: SABR モジュール分離 (Critical)

**現状**: `models/sabr.rs` は以下を含む:
- `SABRParams<T>` - パラメータ構造体
- `SABRModel<T>` - `implied_vol()` (Hagan公式) + `StochasticModel` 実装

**課題**:
1. `SABRModel::implied_vol()` は Monte Carlo とは無関係の解析公式
2. VolCube は `SABRModel` の `implied_vol()` のみを使用
3. 分離すると既存コードの大規模変更が必要

#### ギャップ 2: distributions.rs の扱い

**現状**: 既に単なる re-export
```rust
pub use pricer_core::math::distributions::{norm_cdf, norm_inv_cdf, norm_pdf};
```

**課題**: 削除すると `pricer_models::analytical::distributions::*` を使用するコードが壊れる

---

## 3. 実装アプローチオプション

### Option A: 段階的分離（推奨）

**概要**: SABR を2ファイルに分離、re-export で互換性維持

**変更内容**:
1. `analytical/sabr_implied_vol.rs` を新規作成
   - `SabrImpliedVol<T>` 構造体（Hagan公式）
   - `SABRParams<T>` をここに移動
2. `models/sabr.rs` を簡素化
   - `SABRModel<T>` は `StochasticModel` 実装のみ
   - `analytical::sabr_implied_vol::SABRParams` を使用
3. `models/mod.rs` で後方互換 re-export
   ```rust
   pub use crate::analytical::sabr_implied_vol::{SABRParams, SabrImpliedVol};
   ```

**トレードオフ**:
- ✅ 責務の明確な分離
- ✅ 後方互換性を維持
- ✅ VolCube/キャリブレーションへの影響最小
- ❌ 一時的に2箇所でSABRコードが存在

### Option B: 最小限変更

**概要**: distributions.rs 削除のみ、SABR はそのまま

**変更内容**:
1. `analytical/distributions.rs` を削除
2. `analytical/mod.rs` で `pricer_core` から直接 re-export
3. SABR は現状維持（混合のまま）

**トレードオフ**:
- ✅ 最小限の変更
- ✅ リスクが低い
- ❌ 概念的混乱は解消されない
- ❌ 本来の目的（責務分離）を達成しない

### Option C: 完全分離

**概要**: SABR を完全に分離、非推奨警告を追加

**変更内容**:
1. `analytical/sabr.rs` を新規作成（IV計算のみ）
2. `models/sabr.rs` を MC 専用に簡素化
3. 旧パスに `#[deprecated]` マクロを追加
4. マイグレーションガイドを提供

**トレードオフ**:
- ✅ 最もクリーンな設計
- ✅ 将来の保守性が高い
- ❌ 既存コードへの影響大
- ❌ 移行期間が必要

---

## 4. 複雑性とリスク評価

### 工数見積り

| オプション | 工数 | 理由 |
|------------|------|------|
| Option A | **M (3-5日)** | 適度なリファクタリング、テスト更新 |
| Option B | **S (1-2日)** | 単純な削除と re-export 変更 |
| Option C | **L (1-2週間)** | 広範な変更、deprecation 対応、ドキュメント |

### リスク評価

| オプション | リスク | 理由 |
|------------|--------|------|
| Option A | **Low-Medium** | 既存パターンを活用、互換性維持 |
| Option B | **Low** | 変更が最小限 |
| Option C | **Medium-High** | 既存コード破壊の可能性、移行の複雑さ |

---

## 5. 設計フェーズへの推奨事項

### 推奨アプローチ: Option A（段階的分離）

**理由**:
1. 要件の主要目的（責務分離）を達成
2. 後方互換性を維持しながら移行可能
3. リスクと工数のバランスが良い

### 設計フェーズでの決定事項

1. **SABRParams の配置場所**
   - `analytical/sabr_implied_vol.rs` に移動するか
   - 共通の `models/params.rs` を作成するか

2. **SabrImpliedVol のAPI設計**
   - 既存の `SABRModel::implied_vol()` と同じシグネチャを維持するか
   - より明確な名前（例: `compute_hagan_vol()`）を使うか

3. **distributions.rs の扱い**
   - 削除して直接 `pricer_core` を参照させるか
   - re-export を維持するか（現状維持）

### 研究が必要な項目

| 項目 | 優先度 | 理由 |
|------|--------|------|
| VolCube への影響調査 | High | SABRModel を最も多く使用 |
| キャリブレーションとの整合性 | Medium | SABRCalibrationData との連携 |
| feature flag の影響 | Low | `equity`, `rates` flag の分離状況 |

---

## 6. 結論

### 実装準備状況

| 要件 | 準備状況 | アクション |
|------|----------|----------|
| Req 1 (責務分離) | 設計が必要 | Option A で対応 |
| Req 2 (重複排除) | **実装可能** | distributions.rs 削除 |
| Req 3 (SABR分離) | 設計が必要 | 新規ファイル作成 |
| Req 4 (レイヤー境界) | **OK** | 現状維持 |
| Req 5 (後方互換) | 設計が必要 | re-export 戦略 |
| Req 6 (テスト) | 実装後検証 | 全テスト実行 |

### 次のステップ

1. 設計フェーズで Option A の詳細を策定
2. SABRParams/SabrImpliedVol のAPI詳細を決定
3. 移行パスと後方互換 re-export を明確化
