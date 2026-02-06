# Research & Design Decisions: instrument-precompile

---
**Purpose**: キャリブレーション商品の事前コンパイルに関する調査結果と設計決定を記録する。
---

## Summary
- **Feature**: `instrument-precompile`
- **Discovery Scope**: Extension (既存キャリブレーションシステムの拡張)
- **Key Findings**:
  - 既存の `TradeCompiler` パターンを活用した `InstrumentCompiler` 設計が最適
  - `CompiledInstrument<T>` は SoA (Structure of Arrays) レイアウトを採用
  - 既存の `InterpolationMatrix` を拡張し、CSR 形式は Phase 2 で検討

## Research Log

### TradeCompiler パターンの適用可能性
- **Context**: 既存のコンパイラパターンがキャリブレーション用途に適用可能か調査
- **Sources Consulted**:
  - `pricer_models/src/compiler/mod.rs`
  - `pricer_models/src/compiler/linear.rs`
  - `pricer_core/src/kernel/pricing_kernel.rs`
- **Findings**:
  - `TradeCompiler<T>` トレイトは Trade → PricingKernel 変換を定義
  - `LinearProductsCompiler` は IRS, Bond, FRA, CMS をサポート
  - `PricingKernel` は 64-byte aligned SoA 構造を使用
- **Implications**:
  - 同様のパターンで `InstrumentCompiler` を設計可能
  - ただしキャリブレーション用途では `PricingKernel` より軽量な構造が適切

### 2つの MarketInstrument 型の分析
- **Context**: infra_master と pricer_models に同名の異なる型が存在
- **Sources Consulted**:
  - `infra_master/src/market/market_instrument.rs`
  - `pricer_models/src/market.rs`
- **Findings**:
  - `infra_master::market::MarketInstrument`: Convention + Rate → CF-expandable
  - `pricer_models::market::curves::MarketInstrument<T>`: 軽量 enum (Ois, Irs, Fra, etc.)
  - 前者は `to_trade()` でキャッシュフロー展開、後者は `theoretical_rate()` で評価
- **Implications**:
  - 新しい `CompiledInstrument<T>` 型で両者を橋渡し
  - infra_master 型からのコンパイル、pricer_models 内での評価

### InterpolationMatrix の現状評価
- **Context**: 要件 4 で CSR 形式が要求されている
- **Sources Consulted**:
  - `pricer_models/src/builder/matrix.rs`
  - nalgebra ドキュメント
  - sprs クレート評価
- **Findings**:
  - 現在は Dense DMatrix (nalgebra) を使用
  - `interpolate()` メソッドは O(n×m) の計算量
  - sprs クレートは CSR 形式をサポートするが、nalgebra との互換性に制限あり
- **Implications**:
  - Phase 1: 既存 Dense 形式を維持、最適化は後回し
  - Phase 2: sprs 統合または自前 CSR 実装を検討
  - 現在のカーブキャリブレーション規模では Dense で十分な可能性

### メモリレイアウト選択 (SoA vs AoS)
- **Context**: CompiledInstrument のメモリレイアウト決定
- **Sources Consulted**:
  - `pricer_core/src/kernel/pricing_kernel.rs` (SoA 参考実装)
  - `pricer_core/src/kernel/aligned_buffer.rs`
- **Findings**:
  - SoA: 同種フィールドを連続配置 → SIMD 最適化に有利
  - AoS: 商品単位で配置 → アクセスパターンが単純
  - キャリブレーションでは商品毎の評価が多い → AoS が自然
- **Implications**:
  - `CompiledInstrument<T>` は単一商品の AoS 構造を採用
  - バッチ評価が必要な場合は `CompiledInstrumentSet` を別途検討

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Option A: 既存拡張 | MarketInstrument<T> にフィールド追加 | 最小変更 | 責務混在、後方互換性リスク | 非推奨 |
| Option B: 新規型 | CompiledInstrument<T> を新規作成 | 責務分離、テスト容易 | 新規ファイル追加 | **推奨** |
| Option C: ハイブリッド | Phase 1 で基本、Phase 2 で最適化 | 段階的価値提供 | 複雑な管理 | 大規模向け |

## Design Decisions

### Decision: CompiledInstrument<T> の導入

- **Context**: 2つの MarketInstrument 型の橋渡しと効率的な評価が必要
- **Alternatives Considered**:
  1. Option A: 既存 `MarketInstrument<T>` enum に事前計算フィールドを追加
  2. Option B: 新規 `CompiledInstrument<T>` 型を `pricer_models::builder` に作成
- **Selected Approach**: Option B - 新規 `CompiledInstrument<T>` 型
- **Rationale**:
  - 責務の明確な分離 (定義 vs コンパイル済み)
  - A-I-P-S アーキテクチャとの整合性
  - 既存コードへの影響最小化
- **Trade-offs**:
  - ✅ クリーンな設計、テスト容易性
  - ❌ 新規ファイル追加による管理オーバーヘッド
- **Follow-up**: パフォーマンス目標達成を早期に検証

### Decision: CalibrationInstrument<T> トレイト実装

- **Context**: CompiledInstrument を既存の CalibrationProblem で使用可能にする
- **Alternatives Considered**:
  1. 専用メソッド: `CalibrationProblem::from_compiled()` のみ
  2. トレイト実装: `CompiledInstrument` に `CalibrationInstrument<T>` を実装
- **Selected Approach**: トレイト実装
- **Rationale**:
  - 既存のジェネリック API との互換性維持
  - 将来の拡張性確保
- **Trade-offs**:
  - ✅ 既存 API 再利用可能
  - ❌ トレイト制約の伝播
- **Follow-up**: 型パラメータの明示的アノテーション検証

### Decision: InterpolationMatrix の段階的最適化

- **Context**: CSR 形式への移行コスト vs パフォーマンス改善
- **Alternatives Considered**:
  1. Phase 1 で CSR 形式に完全移行
  2. 段階的アプローチ: Phase 1 は Dense 維持、Phase 2 で CSR 検討
- **Selected Approach**: 段階的アプローチ
- **Rationale**:
  - 現在のカーブキャリブレーション規模 (10-30 商品) では Dense で十分
  - CSR 移行は複雑で、sprs との統合に追加調査が必要
- **Trade-offs**:
  - ✅ 早期の価値提供、リスク軽減
  - ❌ 将来の追加最適化作業
- **Follow-up**: ベンチマークで Dense の性能限界を確認

## Risks & Mitigations

- **後方互換性**: 新規 API 追加のみで対応、既存 API は変更なし → Low Risk
- **パフォーマンス目標未達**: 早期ベンチマーク実施、Dense 形式でも 30% 改善は達成可能と予測 → Medium Risk
- **型変換オーバーヘッド**: コンパイルは 1 回のみ (イテレーション外) → Low Risk
- **テスト破損**: 既存テスト維持、新規テスト追加 → Low Risk

## References

- [pricer_models::compiler](../../crates/pricer_models/src/compiler/mod.rs) — TradeCompiler パターン参照
- [pricer_core::kernel::PricingKernel](../../crates/pricer_core/src/kernel/pricing_kernel.rs) — SoA 設計参照
- [pricer_models::builder::matrix](../../crates/pricer_models/src/builder/matrix.rs) — 既存 InterpolationMatrix
- [infra_master::market::MarketInstrument](../../crates/infra_master/src/market/market_instrument.rs) — CF-expandable 型

---

_生成日: 2026-02-06_
_ドキュメントバージョン: 1.0_
