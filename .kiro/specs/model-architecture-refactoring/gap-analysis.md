# ギャップ分析: model-architecture-refactoring

## 概要

本ドキュメントは、`pricer_models`と`pricer_optimiser`間のモデル・キャリブレーション機能の重複を解消するリファクタリングのギャップ分析結果である。

---

## 1. 現状調査

### 1.1 キャリブレーション実装の比較

| 場所 | 行数 | 内容 |
|------|------|------|
| `pricer_models/src/calibration/` | **4,081行** | 完全なキャリブレーター実装 |
| `pricer_optimiser/src/calibration/` | **211行** | 簡易CalibrationEngine |

**pricer_models/src/calibration/** の内訳:
- `heston.rs` (1,165行): Heston特性関数プライシング + キャリブレーター
- `sabr.rs` (750行): SABRキャリブレーター (Hagan公式)
- `hull_white.rs` (506行): Hull-Whiteスワップションキャリブレーター
- `swaption_calibrator.rs` (525行): 汎用スワップションキャリブレーター
- `model_calibrator.rs` (444行): LMソルバーラッパー（`pricer_core`のLMを使用）
- `result.rs` (222行): CalibrationResult, CalibrationDiagnostics
- `error.rs` (212行): CalibrationError
- `targets.rs` (190行): CalibrationTarget型

**pricer_optimiser/src/calibration/** の内訳:
- `engine.rs` (178行): 簡易CalibrationEngine（独自FD実装）
- `mod.rs` (33行): CalibrationMarketData

### 1.2 LevenbergMarquardtソルバーの重複

| 場所 | 使用目的 |
|------|----------|
| `pricer_core/src/math/solvers/levenberg_marquardt.rs` | `LevenbergMarquardtSolver` - pricer_modelsのキャリブレーターが使用 |
| `pricer_optimiser/src/solvers/levenberg_marquardt.rs` | `LevenbergMarquardt` - 別実装（独自） |

**発見**: LMソルバーが2箇所に存在。`pricer_models`のキャリブレーターは`pricer_core`のLMを使用している。

### 1.3 モデル定義の配置

**現在のmodels/構造**:
```
models/
├── gbm.rs          (276行) - ルートレベル
├── heston.rs       (2,670行) - ルートレベル
├── sabr.rs         (2,886行) - ルートレベル
├── stochastic.rs   (504行) - trait定義
├── model_enum.rs   (909行) - StochasticModelEnum
├── mod.rs          (68行)
├── equity/
│   └── mod.rs      (22行) - 空に近い
├── rates/
│   ├── hull_white.rs (805行)
│   ├── cir.rs      (467行)
│   └── mod.rs      (33行)
└── hybrid/
    ├── correlated.rs (704行)
    └── mod.rs      (41行)
```

**問題点**:
- GBM, Heston, SABRがルートレベルに散在
- `equity/mod.rs`はほぼ空で実質未使用
- 株式系モデル（GBM, Heston, SABR）と金利系モデル（Hull-White, CIR）の分離が不完全

### 1.4 依存関係の確認

```
pricer_core (L1)
    ↑
pricer_models (L2) [pricer_optimiserに依存していない ✓]
    ↑
pricer_optimiser (L2.5) [pricer_modelsに依存している ✓]
```

**依存方向は正しい**。循環依存のリスクなし。

### 1.5 外部使用状況

キャリブレーターの使用箇所を検索した結果:
- `pricer_risk`: 使用なし
- `demo/`: 使用なし
- `service_*`: 使用なし

**結論**: キャリブレーターのpublic APIは現在外部から使用されていない。後方互換性の懸念は低い。

---

## 2. 要件実現可能性分析

### 2.1 Requirement 1: キャリブレーション機能の統合

| 技術要件 | 現状 | ギャップ |
|----------|------|----------|
| Heston/SABR/HWキャリブレーターの移動 | pricer_modelsに存在 | **移動が必要** |
| pricer_optimiserのCalibrationEngine強化 | 簡易版のみ | **削除して置換** |
| 後方互換re-export | なし | **新規作成** (ただし使用者なし) |

**実装複雑度**: 中 - ファイル移動とimport修正が主

### 2.2 Requirement 2: モデル構造の整理

| 技術要件 | 現状 | ギャップ |
|----------|------|----------|
| GBM/Heston/SABRをequity/に移動 | ルートレベル | **移動が必要** |
| re-exportの維持 | なし | **新規作成** |
| feature flagゲート | 部分的 | **調整が必要** |

**実装複雑度**: 中 - `model_enum.rs`のimport更新が複雑

### 2.3 Requirement 3: pricer_optimiserの強化

| 技術要件 | 現状 | ギャップ |
|----------|------|----------|
| 統一キャリブレーションAPI | 2つの異なるAPI | **統合が必要** |
| LMソルバーの選択 | 2つ存在 | **pricer_coreを使用** |

**研究必要項目**: pricer_optimiserの独自LM実装を削除するか、維持するか要検討

### 2.4 Requirement 4: 依存関係の整理

| 技術要件 | 現状 | ギャップ |
|----------|------|----------|
| L1←L2←L2.5の維持 | 正しい | **変更不要** |
| 循環依存の回避 | なし | **維持** |

### 2.5 Requirement 5-6: ドキュメント・後方互換性

| 技術要件 | 現状 | ギャップ |
|----------|------|----------|
| steering文書更新 | 必要 | **更新が必要** |
| deprecation警告 | なし | **使用者がいないため優先度低** |

---

## 3. 実装アプローチオプション

### Option A: 完全移動（推奨）

**内容**: `pricer_models/src/calibration/`を丸ごと`pricer_optimiser/src/calibration/`に移動

**手順**:
1. pricer_models/src/calibration/*.rsをpricer_optimiser/src/calibration/に移動
2. import文を`pricer_models::`から`pricer_core::`と`pricer_models::models::`に更新
3. pricer_modelsからcalibrationモジュールを削除
4. pricer_optimiserの既存engine.rsを削除（ModelCalibratorに置換）
5. models/内のHeston, SABR, GBMをequity/に移動
6. re-exportを追加してAPI互換性を維持

**トレードオフ**:
- ✅ アーキテクチャ設計との完全一致
- ✅ 責務の明確な分離
- ✅ 将来のメンテナンス容易性
- ❌ 多数のファイル移動と修正
- ❌ テストのimport修正が必要

### Option B: 段階的移行

**内容**: 新機能はpricer_optimiserに追加、既存はdeprecated

**手順**:
1. pricer_optimiserに新しいキャリブレーターインターフェースを作成
2. 既存のpricer_models::calibrationをdeprecatedとしてマーク
3. 段階的に移行

**トレードオフ**:
- ✅ 低リスク、段階的
- ✅ 即座のブレーキングチェンジなし
- ❌ 一時的な重複状態が継続
- ❌ 完了までの期間が長い

### Option C: インターフェース統合のみ

**内容**: コードは移動せず、re-exportとfacadeで統合

**手順**:
1. pricer_optimiserからpricer_models::calibrationをre-export
2. ユーザーAPIはpricer_optimiser経由に統一

**トレードオフ**:
- ✅ 最小限の変更
- ❌ 実際のコード配置は改善されない
- ❌ アーキテクチャ設計との乖離が継続

---

## 4. 追加発見事項

### 4.1 LMソルバーの重複（要決定）

`pricer_core`と`pricer_optimiser`の両方にLM実装がある。

**オプション**:
1. `pricer_optimiser`の独自LMを削除し、`pricer_core`のLMに統一
2. 両方維持（異なるユースケース用）

**推奨**: オプション1（統一）。`pricer_models`のキャリブレーターはすでに`pricer_core`のLMを使用しているため。

### 4.2 モデルのfeature flag整理

現在のfeature flags:
- `equity` (default)
- `rates`
- `credit`
- `fx`
- `commodity`
- `exotic`

Heston, SABRは現在feature flagなしでルートレベルに存在。`exotic`または`equity`のゲート下に移動すべきか要検討。

**推奨**: Heston, SABRは広く使用される基本モデルなので`equity`（default）に含める。

---

## 5. 複雑度とリスク評価

### 工数見積もり

| タスク | 工数 | 根拠 |
|--------|------|------|
| キャリブレーション移動 | M (3-5日) | 4,000行以上、import修正多数 |
| モデル構造整理 | M (2-3日) | 6,000行以上、model_enum.rs修正複雑 |
| LMソルバー統合 | S (1日) | 削除と置換のみ |
| ドキュメント更新 | S (1日) | steering/structure.md更新 |
| テスト修正・確認 | M (2-3日) | 全キャリブレーションテスト確認 |

**合計**: **L (7-10日)**

### リスク評価

| リスク | 評価 | 軽減策 |
|--------|------|--------|
| ビルド失敗 | 低 | 段階的にビルド確認 |
| テスト失敗 | 中 | 移動前後でテスト比較 |
| API破壊 | 低 | 現在外部使用なし |
| 循環依存 | 低 | 既に正しい方向 |

**総合リスク**: **Low-Medium**

---

## 6. 設計フェーズへの推奨事項

### 推奨アプローチ

**Option A: 完全移動**を推奨

### 主要な設計決定事項

1. **LMソルバー**: `pricer_optimiser`の独自実装を削除し、`pricer_core::math::solvers::LevenbergMarquardtSolver`に統一
2. **モデル配置**: Heston, SABR, GBMを`models/equity/`に移動
3. **後方互換性**: 使用者が確認されないため、deprecation期間は短く（1マイナーバージョン）

### 研究項目（設計フェーズで検討）

1. `model_enum.rs`のリファクタリング詳細（9,000行のファイルを分割するか？）
2. キャリブレーターのトレイト設計（`Calibrator<M>`ジェネリクスの活用）
3. 移行順序（キャリブレーション先 vs モデル構造先）
