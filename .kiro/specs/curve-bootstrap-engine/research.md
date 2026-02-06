# Research & Design Decisions: curve-bootstrap-engine

## Summary
- **Feature**: `curve-bootstrap-engine`
- **Discovery Scope**: Extension（既存システムの拡張）
- **Key Findings**:
  - 既存の`pricer_models/src/market/calibration/bootstrapping/`モジュールに70-80%の基盤が存在
  - 主要ギャップ: Index-Curve定義、infra_domain統合、結果キャッシュの3領域
  - A-I-P-S依存ルールに準拠するため、Adapter層ではなくPricer層内で統合を実現

## Research Log

### Topic: 既存Bootstrapモジュールの成熟度評価

- **Context**: 要件を満たすために既存コードをどの程度再利用できるか評価
- **Sources Consulted**:
  - [bootstrapping/mod.rs](crates/pricer_models/src/market/calibration/bootstrapping/mod.rs)
  - [bootstrapping/engine.rs](crates/pricer_models/src/market/calibration/bootstrapping/engine.rs)
  - [bootstrapping/curve.rs](crates/pricer_models/src/market/calibration/bootstrapping/curve.rs)
- **Findings**:
  - `SequentialBootstrapper<T>`: Newton-Raphson + Brent fallback完備（Req 4 充足）
  - `BootstrappedCurve<T>`: YieldCurveトレイト実装済み（Req 5 部分充足）
  - `MultiCurveBuilder<T>`: OIS Discount + Tenor Curve構築済み（Req 8 充足）
  - `SensitivityBootstrapper`: Jacobian計算済み（Req 6 部分充足）
  - `BootstrapError`: thiserror構造化エラー完備（Req 9 充足）
- **Implications**: 新規実装は3領域（定義層、統合層、キャッシュ層）に限定可能

### Topic: infra_domainとの統合アプローチ

- **Context**: A-I-P-S依存ルールを維持しながら`infra_domain::trade`の型を活用する方法
- **Sources Consulted**:
  - [steering/structure.md](.kiro/steering/structure.md) - 依存ルール
  - [infra_domain/trade/index.rs](crates/infra_domain/src/trade/index.rs)
  - [infra_domain/trade/convention/swap.rs](crates/infra_domain/src/trade/convention/swap.rs)
- **Findings**:
  - **依存ルール**: Pricerクレートは**I**nfraに依存可能（`pricer_models` → `infra_domain`はOK）
  - `RateIndex`: SOFR, Euribor3M/6M, Tonar, Sonia, Tibor, Estr定義済み
  - `SwapConvention`: 通貨別コンベンション（usd_sofr, eur_euribor_6m等）定義済み
  - `InstrumentExpander`: 商品定義からトレード展開機能あり
- **Implications**:
  - `pricer_models`から`infra_domain`への依存追加は許可される
  - Adapter層新設は不要、Pricer層内でブリッジを実装

### Topic: LRUキャッシュライブラリ選定

- **Context**: スレッドセーフなLRUキャッシュ実装の選択
- **Sources Consulted**: Rust ecosystem調査
- **Findings**:
  - **`lru` crate**: 標準的なLRU実装、`std::sync::RwLock`でラップ可能
  - **`quick_cache`**: 並行アクセス最適化、内部でsharded lockingを使用
  - **`dashmap`**: 汎用並行HashMap、LRU機能なし
  - 既存の`BufferPool`は`RefCell`使用でスレッドセーフではない
- **Implications**:
  - `lru` + `parking_lot::RwLock`を推奨（シンプルかつ十分な性能）
  - 高並行環境では`quick_cache`への移行を検討

### Topic: パラメータ表現（LogDF vs ZeroRate）の内部実装

- **Context**: カーブ内部表現の抽象化方法
- **Sources Consulted**:
  - [bootstrapping/config.rs](crates/pricer_models/src/market/calibration/bootstrapping/config.rs)
  - [bootstrapping/curve.rs](crates/pricer_models/src/market/calibration/bootstrapping/curve.rs)
- **Findings**:
  - 現在の`BootstrappedCurve`は内部的にLogDFを格納
  - 補間は`BootstrapInterpolation`で選択可能（LogLinear, LinearZeroRate等）
  - パラメータ表現の切り替えは補間メソッドとdiscount_factor計算に影響
- **Implications**:
  - `CurveParameterRepresentation` enumを追加
  - `BootstrappedCurve`の内部構造を抽象化するのではなく、変換レイヤーで対応

### Topic: Dual型互換性

- **Context**: `pricer_core::types::Dual`でのカーブ構築可否
- **Sources Consulted**:
  - [bootstrapping/sensitivity.rs](crates/pricer_models/src/market/calibration/bootstrapping/sensitivity.rs)
  - [pricer_core/src/types/dual.rs](crates/pricer_core/src/types/dual.rs)
- **Findings**:
  - `BootstrappedCurve<T: Float>`はジェネリック対応済み
  - `SensitivityBootstrapper`は`f64`専用（Implicit Function Theorem使用）
  - `num-dual-mode` featureでDual型のインスタンス化は理論上可能
  - ソルバーのNewton-Raphson/Brentは`Float`バウンドで動作
- **Implications**:
  - `BootstrappedCurve<Dual>`のインスタンス化テストが必要
  - 感度計算は既存のIFT方式を維持（Dual tapeへの記録は不要）

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| **A: 既存モジュール拡張** | `bootstrapping/`に新ファイル追加 | 既存テスト活用、影響最小、A-I-P-S準拠 | モジュール肥大化 | **推奨** |
| B: 新規サブモジュール | `curve_builder/`新設 | クリーン設計 | 既存コードとの重複 | 却下 |
| C: Adapterクレート | `adapter_curve`新設 | 責務分離 | クレート数増加、ビルド時間 | 却下 |

## Design Decisions

### Decision: 既存bootstrapモジュールの拡張

- **Context**: Index-Curve定義、統合層、キャッシュ層の配置先選定
- **Alternatives Considered**:
  1. Option A - `bootstrapping/`モジュール内に新ファイル追加
  2. Option B - `market/curve_builder/`として新規モジュール作成
  3. Option C - `adapter_curve`クレート新設
- **Selected Approach**: Option A - 既存モジュール拡張
- **Rationale**:
  - A-I-P-S依存ルールに完全準拠
  - 既存の`SequentialBootstrapper`, `BootstrappedCurve`を直接活用
  - テスト・ドキュメントの一貫性維持
- **Trade-offs**: モジュールサイズ増加を許容、ただし責務ごとにファイル分割で管理
- **Follow-up**: 新規ファイル数が5を超える場合はサブモジュール化を検討

### Decision: Implicit Function Theoremによる感度計算維持

- **Context**: AD対応の計算グラフ設計（Req 6）
- **Alternatives Considered**:
  1. 現在のIFT方式を維持（Jacobian行列として感度を保持）
  2. Forward-mode ADでソルバー反復をテープに記録
  3. Reverse-mode ADで全計算グラフを記録
- **Selected Approach**: IFT方式を維持
- **Rationale**:
  - ソルバー反復をAD tapeに記録すると計算コストが指数的に増加
  - IFTは収束点での暗黙関数微分により効率的
  - 既存`SensitivityBootstrapper`が検証済み
- **Trade-offs**: 明示的な計算グラフは保持せず、Jacobian行列で代替
- **Follow-up**: Enzyme統合テストで性能検証

### Decision: parking_lot::RwLockによるスレッドセーフキャッシュ

- **Context**: 結果キャッシュのスレッドセーフ実装（Req 7）
- **Alternatives Considered**:
  1. `std::sync::RwLock<lru::LruCache>`
  2. `parking_lot::RwLock<lru::LruCache>`
  3. `quick_cache::sync::Cache`
  4. `dashmap` + 手動LRU管理
- **Selected Approach**: `parking_lot::RwLock<lru::LruCache>`
- **Rationale**:
  - `parking_lot`は`std::sync`より高速（特に低競合時）
  - `lru`クレートはシンプルで十分なLRU機能を提供
  - 既存ワークスペースで`parking_lot`使用実績あり
- **Trade-offs**: 高並行環境では`quick_cache`が優位な可能性
- **Follow-up**: 並列ベンチマークで競合時の性能検証

### Decision: CurveKeyのハッシュ設計

- **Context**: キャッシュキーの一意性保証（Req 7.1）
- **Alternatives Considered**:
  1. `(RateIndex, Vec<f64>, GenericBootstrapConfig)` の直接ハッシュ
  2. `(RateIndex, u64, u64)` - rates配列とconfigのハッシュ値
  3. `(String, u64)` - Index文字列 + 複合ハッシュ
- **Selected Approach**: `(RateIndex, u64, u64)` 形式
- **Rationale**:
  - `Vec<f64>`の直接ハッシュは浮動小数点比較の問題あり
  - `RateIndex`はCopy+Eq+Hashを満たす
  - `f64`配列は`ordered-float`でHashable化、またはビット表現でハッシュ
- **Trade-offs**: ハッシュ衝突リスク（極めて低い）
- **Follow-up**: `ordered-float`クレートの追加または`f64::to_bits()`でのハッシュ実装

## Risks & Mitigations

- **Risk 1: Dual型互換性** — `BootstrappedCurve<Dual>`の構築テストをPhase 5で実施。失敗時はジェネリック制約を緩和またはf64専用パスを提供
- **Risk 2: キャッシュメモリ使用量** — デフォルトLRUサイズを100エントリに制限。設定可能とし運用で調整
- **Risk 3: RwLock競合** — 高並行ワークロードでの性能劣化をベンチマークで監視。必要に応じて`quick_cache`へ移行
- **Risk 4: infra_domain依存追加** — Cargo.tomlで`infra_domain`を`pricer_models`の依存に追加。A-I-P-Sルール上は許可される

## References

- [steering/structure.md](.kiro/steering/structure.md) — A-I-P-S依存ルール
- [steering/tech.md](.kiro/steering/tech.md) — 技術スタック
- [steering/error-handling.md](.kiro/steering/error-handling.md) — thiserrorパターン
- [lru crate](https://docs.rs/lru) — LRUキャッシュ実装
- [parking_lot](https://docs.rs/parking_lot) — 高速RwLock
- [gap-analysis.md](.kiro/specs/curve-bootstrap-engine/gap-analysis.md) — 詳細ギャップ分析
