# Gap Analysis: curve-bootstrap-engine

## 1. 分析サマリー

### スコープ
本仕様は、Index単位のカーブ定義、Bootstrap エンジン、汎用カーブインターフェース、AD対応計算グラフ、結果キャッシュを含む包括的なカーブ構築システムを要求している。

### 既存コードベースの成熟度
**High（70-80%）** - `pricer_models/src/market/calibration/bootstrapping/`モジュールに強力な基盤が存在する。

### 主要なギャップ
1. **Index-Curve Definition**: Index→Instrument集合のマッピングが未実装
2. **infra_domain統合**: `BootstrapInstrument`が`infra_domain::trade`と非統合
3. **結果キャッシュ**: LRUベースの構築済みカーブキャッシュが未実装
4. **設定シリアライゼーション**: serde対応が部分的

### 推奨アプローチ
**ハイブリッド（拡張 + 新規）** - 既存Bootstrapモジュールを活用しつつ、新規のIndex-Curve定義層とキャッシュ層を追加する。

## 2. 要件別ギャップ詳細

### Requirement 1: Index-Curve Definition（Index別カーブ定義）

**ギャップレベル**: HIGH - 新規実装が必要

**既存コード**: `infra_domain::trade::index.rs` に `IndexType`, `RateIndex` が定義済み

**ギャップ**: Index → 必要Instrument集合のマッピングが存在しない、テナーポイント定義の仕組みがない

**統合ポイント**: `SwapConvention::usd_sofr()`, `SwapConvention::eur_euribor_6m()` 等の既存コンベンション

**設計考慮事項**: 新規 `CurveDefinition`, `InstrumentSpec` 構造体の作成が必要

### Requirement 2: Curve Parameter Configuration（カーブパラメータ設定）

**ギャップレベル**: MEDIUM - 拡張が必要

**既存コード**: `BootstrapInterpolation` enum (LogLinear, LinearZeroRate, CubicSpline, MonotonicCubic, FlatForward) が存在

**ギャップ**: パラメータ表現種別（LogDF, ZeroRate, InstantaneousForward）が未定義、外挿設定の詳細オプションが限定的

**統合ポイント**: `GenericBootstrapConfig<T>` に新フィールド追加、`BootstrappedCurve<T>` の内部表現を抽象化

### Requirement 3: Instrument-to-Cashflow Integration（Instrument-キャッシュフロー統合）

**ギャップレベル**: HIGH - ブリッジ層の新規実装が必要

**既存コード分離状態**:
- pricer_models側: `BootstrapInstrument<T>` enum (Ois, Irs, Fra, Future variants)
- infra_domain側: `SwapConvention`, `SwapLegConvention`

**ギャップ**: `BootstrapInstrument`は`infra_domain`のコンベンションを使用していない、キャッシュフロー展開との連携がない

**必要な作業**: `infra_domain::trade::instrument_def` → `BootstrapInstrument` 変換器、`SwapConvention`からキャッシュフロースケジュール生成

### Requirement 4: Bootstrap Engine（ブートストラップエンジン）

**ギャップレベル**: LOW - 既存実装で十分

**既存コード**: `SequentialBootstrapper<T>` - Newton-Raphson + Brent fallback完備

**完全に実装済み**: Newton-Raphson法、Brent法フォールバック、収束許容誤差・最大反復回数設定、残差・収束ステータス返却

### Requirement 5: Generic Curve Interface（汎用カーブインターフェース）

**ギャップレベル**: LOW - 軽微な拡張のみ

**既存コード**: `YieldCurve<T>` trait (discount_factor, zero_rate, forward_rate)

**ギャップ**: `instantaneous_forward(t)` メソッドが未定義、pillar点アクセサがトレイトレベルで未定義

**必要な作業**: `YieldCurve`トレイトに`instantaneous_forward()`追加

### Requirement 6: Computation Graph for AD（自動微分用計算グラフ）

**ギャップレベル**: MEDIUM - 検証・拡張が必要

**既存コード**: `SensitivityBootstrapper` (Implicit Function Theorem), `BootstrapResultWithSensitivities` (Jacobian保持)

**ギャップ**: `pricer_core::types::Dual`との互換性検証が必要、ジェネリックな`SensitivityBootstrapper<T>`の実装検討

**設計選択肢**: 現在のImplicit Function Theorem方式を維持（推奨）

### Requirement 7: Curve Caching（カーブキャッシュ）

**ギャップレベル**: HIGH - 新規実装が必要

**既存コード**: `CurveCache<T>` (内部メモリ最適化用、結果キャッシュではない)

**ギャップ**: 結果キャッシュが存在しない、LRUエビクション機構がない、キャッシュキー設計がない

**必要な新規コンポーネント**: `CurveResultCache<T>` with `Arc<RwLock<LruCache>>`, `CurveKey` (Index + rates_hash + config_hash)

### Requirement 8: Multi-Curve Support（マルチカーブ対応）

**ギャップレベル**: LOW - 既存実装で十分

**既存コード**: `MultiCurveBuilder<T>` (OIS Discount + Tenor Curve), `CurveSet<T>`

**軽微なギャップ**: 依存関係の自動解決（現在は手動）、循環依存検出の明示的エラー

### Requirement 9: Error Handling（エラーハンドリング）

**ギャップレベル**: NONE - 完全実装済み

**既存コード**: `BootstrapError` enum (`thiserror`, ConvergenceFailure with details)

### Requirement 10: Configuration Serialization（設定のシリアライゼーション）

**ギャップレベル**: MEDIUM - serde対応の追加が必要

**必要な作業**: `GenericBootstrapConfig<T>` に `Serialize/Deserialize` 追加、`BootstrapInterpolation` に serde derive追加

## 3. 実装アプローチ選択肢

### Option A: 既存モジュール拡張（推奨）

**概要**: `pricer_models/src/market/calibration/bootstrapping/` を拡張

**メリット**: 既存のテスト・ドキュメント活用、A-I-P-S準拠、既存ユーザーへの影響最小

**デメリット**: `infra_domain`との統合にAdapterパターンが必要

**新規ファイル**:
```
crates/pricer_models/src/market/calibration/bootstrapping/
├── definition.rs    # CurveDefinition, InstrumentSpec
├── result_cache.rs  # CurveResultCache (LRU)
└── adapter.rs       # infra_domain → BootstrapInstrument 変換
```

## 4. 技術的リスクと調査項目

**Risk 1: Dual型互換性** — `BootstrappedCurve<Dual>`のインスタンス化テストが必要

**Risk 2: キャッシュのメモリ使用量** — カーブ1本あたりのメモリフットプリント測定、LRUサイズ制限

**Risk 3: スレッドセーフキャッシュのパフォーマンス** — `RwLock`のcontentionベンチマーク、`dashmap`検討

## 5. 推奨実装順序

1. **Phase 1: 基盤拡張** — `CurveDefinition`, `InstrumentSpec` 型定義、`GenericBootstrapConfig` へのparameter representation追加、serde対応

2. **Phase 2: 統合層** — `infra_domain` → `BootstrapInstrument` アダプター、`SwapConvention`からのキャッシュフロー展開

3. **Phase 3: キャッシュ実装** — `CurveResultCache` (LRU, thread-safe)、キャッシュキー設計

4. **Phase 4: インターフェース拡張** — `YieldCurve`トレイトに`instantaneous_forward()`追加、Multi-curve依存関係自動解決

5. **Phase 5: 検証・最適化** — `Dual`型互換性テスト、パフォーマンスベンチマーク

## 6. 参照ファイル一覧

| ファイル | 関連要件 | 状態 |
|----------|----------|------|
| bootstrapping/mod.rs | 全般 | 既存 |
| bootstrapping/config.rs | Req 2, 10 | 拡張必要 |
| bootstrapping/instrument.rs | Req 3 | 統合必要 |
| bootstrapping/engine.rs | Req 4 | Complete |
| bootstrapping/curve.rs | Req 5 | 軽微拡張 |
| bootstrapping/multi_curve.rs | Req 8 | Complete |
| bootstrapping/sensitivity.rs | Req 6 | 検証必要 |
| bootstrapping/cache.rs | Req 7 | 新規必要 |
| bootstrapping/error.rs | Req 9 | Complete |
| market/curves/traits.rs | Req 5 | 軽微拡張 |
| infra_domain/trade/index.rs | Req 1 | 参照 |
| infra_domain/trade/convention/swap.rs | Req 1, 3 | 参照 |
| infra_domain/trade/cashflow.rs | Req 3 | 参照 |
