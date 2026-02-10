# Research & Design Decisions: curve-bootstrap-engine

## Summary
- **Feature**: `curve-bootstrap-engine`
- **Discovery Scope**: Extension（既存システムの拡張）
- **Key Findings**:
  - 既存の`pricer_models/src/market/calibration/bootstrapping/`モジュールに70-80%の基盤が存在
  - 主要ギャップ: Index-Curve定義、infra_domain統合、結果キャッシュの3領域
  - A-I-P-S依存ルールに準拠するため、Adapter層ではなくPricer層内で統合を実現

## Research Log

### 既存Bootstrapモジュールの成熟度評価

**Findings**:
- `SequentialBootstrapper<T>`: Newton-Raphson + Brent fallback完備（Req 4 充足）
- `BootstrappedCurve<T>`: YieldCurveトレイト実装済み（Req 5 部分充足）
- `MultiCurveBuilder<T>`: OIS Discount + Tenor Curve構築済み（Req 8 充足）
- `SensitivityBootstrapper`: Jacobian計算済み（Req 6 部分充足）
- `BootstrapError`: thiserror構造化エラー完備（Req 9 充足）

**Implications**: 新規実装は3領域（定義層、統合層、キャッシュ層）に限定可能

### infra_domainとの統合アプローチ

**Findings**:
- **依存ルール**: Pricerクレートは**I**nfraに依存可能（`pricer_models` → `infra_domain`はOK）
- `RateIndex`: SOFR, Euribor3M/6M, Tonar, Sonia, Tibor, Estr定義済み
- `SwapConvention`: 通貨別コンベンション（usd_sofr, eur_euribor_6m等）定義済み
- `InstrumentExpander`: 商品定義からトレード展開機能あり

**Implications**: `pricer_models`から`infra_domain`への依存追加は許可される、Adapter層新設は不要

### LRUキャッシュライブラリ選定

**Findings**:
- **`lru` crate**: 標準的なLRU実装、`std::sync::RwLock`でラップ可能
- **`quick_cache`**: 並行アクセス最適化 — 高並行環境での移行を検討
- **`dashmap`**: 汎用並行HashMap — LRU機能なし

**Implications**: `lru` + `parking_lot::RwLock`を推奨（シンプルかつ十分な性能）

### パラメータ表現（LogDF vs ZeroRate）の内部実装

**Findings**:
- 現在の`BootstrappedCurve`は内部的にLogDFを格納
- 補間は`BootstrapInterpolation`で選択可能（LogLinear, LinearZeroRate等）
- パラメータ表現の切り替えは補間メソッドとdiscount_factor計算に影響

**Implications**: `CurveParameterRepresentation` enumを追加、変換レイヤーで対応

### Dual型互換性

**Findings**:
- `BootstrappedCurve<T: Float>`はジェネリック対応済み
- `SensitivityBootstrapper`は`f64`専用（Implicit Function Theorem使用）
- `num-dual-mode` featureでDual型のインスタンス化は理論上可能
- ソルバーのNewton-Raphson/Brentは`Float`バウンドで動作

**Implications**: `BootstrappedCurve<Dual>`のインスタンス化テストが必要、感度計算は既存のIFT方式を維持

## Architecture Pattern Evaluation

| Option | Strengths | Risks / Limitations |
|--------|-----------|---------------------|
| **A: 既存モジュール拡張** | 既存テスト活用、影響最小、A-I-P-S準拠 | モジュール肥大化 |
| B: 新規サブモジュール | クリーン設計 | 既存コードとの重複 |
| C: Adapterクレート | 責務分離 | クレート数増加、ビルド時間 |

**Selected**: Option A - 既存モジュール拡張

## Design Decisions

### Decision: 既存bootstrapモジュールの拡張

**Selected Approach**: Option A - 既存モジュール拡張

**Rationale**: A-I-P-S依存ルールに完全準拠、既存の`SequentialBootstrapper`, `BootstrappedCurve`を直接活用

**Alternatives**: Option B (新規モジュール) - コード重複、Option C (Adapterクレート) - クレート数増加

### Decision: Implicit Function Theoremによる感度計算維持

**Selected Approach**: IFT方式を維持

**Rationale**: ソルバー反復をAD tapeに記録すると計算コスト指数的増加、IFTは収束点での暗黙関数微分により効率的

**Alternatives**: Forward-mode ADでソルバー反復を記録 - 計算コスト高、Reverse-mode AD - 全計算グラフ記録必要

### Decision: parking_lot::RwLockによるスレッドセーフキャッシュ

**Selected Approach**: `parking_lot::RwLock<lru::LruCache>`

**Rationale**: `parking_lot`は`std::sync`より高速、`lru`クレートはシンプルで十分、既存ワークスペースで使用実績あり

**Alternatives**: `quick_cache` (高並行環境で有利だが複雑), `dashmap` (LRU機能なし)

### Decision: CurveKeyのハッシュ設計

**Selected Approach**: `(RateIndex, u64, u64)` 形式

**Rationale**: `Vec<f64>`の直接ハッシュは浮動小数点比較の問題あり、`RateIndex`はCopy+Eq+Hash、`ordered-float`でHashable化

**Alternatives**: 直接ハッシュ (浮動小数点問題), String形式 (衝突リスク)

## Risks & Mitigations

- **Risk 1: Dual型互換性** — `BootstrappedCurve<Dual>`の構築テストをPhase 5で実施
- **Risk 2: キャッシュメモリ使用量** — デフォルトLRUサイズを100エントリに制限、設定可能とし運用で調整
- **Risk 3: RwLock競合** — 高並行ワークロードでの性能劣化をベンチマークで監視、`quick_cache`への移行検討
- **Risk 4: infra_domain依存追加** — Cargo.tomlで`infra_domain`を`pricer_models`の依存に追加（A-I-P-Sルール上許可）

## References

- [steering/structure.md](.kiro/steering/structure.md) — A-I-P-S依存ルール
- [steering/tech.md](.kiro/steering/tech.md) — 技術スタック
- [lru crate](https://docs.rs/lru) — LRUキャッシュ実装
- [parking_lot](https://docs.rs/parking_lot) — 高速RwLock
- [gap-analysis.md](.kiro/specs/curve-bootstrap-engine/gap-analysis.md) — 詳細ギャップ分析
