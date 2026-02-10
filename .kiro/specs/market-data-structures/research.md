# Research & Design Decisions

## Summary
- **Feature**: `market-data-structures`
- **Discovery Scope**: Extension (adding to existing pricer_core crate)
- **Key Findings**:
  - 既存の `Interpolator<T: Float>` trait パターンを YieldCurve / VolatilitySurface に適用可能
  - `LinearInterpolator`, `BilinearInterpolator` が既に実装済み - 再利用可能
  - `InterpolationError` enum が存在 - MarketDataError として拡張または統合

## Research Log

### Existing Interpolation Infrastructure
- **Context**: 要件3, 6で既存補間器との統合が必要
- **Findings**:
  - `Interpolator<T: Float>` trait: `interpolate(&self, x: T) -> Result<T, InterpolationError>`, `domain(&self) -> (T, T)`
  - `LinearInterpolator<T>`: 1D線形補間、ソート済みデータ点
  - `BilinearInterpolator<T>`: 2Dグリッド補間、Vol Surface に最適
  - Binary search による O(log n) セグメント検索
- **Implications**: YieldCurve は Interpolator を内部で利用、VolSurface は BilinearInterpolator をラップ

### Error Handling Patterns
- **Context**: 要件7で統一エラーハンドリングが必要
- **Findings**:
  - `InterpolationError`: OutOfBounds, InsufficientData, NonMonotonicData, InvalidInput
  - `PricingError`: InvalidInput, NumericalInstability, ModelFailure, UnsupportedInstrument
  - `thiserror` を使用した derive macro パターン
- **Implications**: `MarketDataError` を新規追加し、`InterpolationError` からの変換 trait を実装

### Generic Type System
- **Context**: 要件8でDual64互換性が必要
- **Findings**:
  - `pub use num_traits::Float;` で Float trait を re-export
  - 全 interpolator が `T: Float` で generic
  - `smooth_max`, `smooth_abs` が AD 互換の分岐回避を提供
- **Implications**: 全 market data struct を `T: Float` でパラメータ化、分岐は smoothing 関数で回避

## Design Decisions

### Decision: YieldCurve Trait API Design
- **Selected Approach**: `discount_factor` を必須、`zero_rate` と `forward_rate` はデフォルト実装
- **Rationale**: discount_factor から他の量は導出可能、実装負荷軽減

### Decision: Error Type Integration
- **Selected Approach**: 新規 MarketDataError を追加、From trait で InterpolationError から変換
- **Rationale**: 市場データ固有のエラー（InvalidMaturity, InvalidStrike）を明確に区別

### Decision: InterpolatedCurve Interpolation Strategy
- **Selected Approach**: `InterpolationMethod` enum (Linear, LogLinear, CubicSpline)
- **Rationale**: 実行時に補間方式を切り替え可能、API シンプル

## Risks & Mitigations
- **Risk 1**: 補間器の数値安定性（特に log-linear） — 小さい discount factor に対する ln() のオーバーフローチェック追加
- **Risk 2**: Vol Surface の extrapolation boundary — flat extrapolation をデフォルトとし、設定可能に
- **Risk 3**: Dual64 での AD tape 一貫性 — smoothing 関数を活用、分岐を回避
