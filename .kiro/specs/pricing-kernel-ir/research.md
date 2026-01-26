# Research & Design Decisions: pricing-kernel-ir

## Summary

- **Feature**: `pricing-kernel-ir`
- **Discovery Scope**: Complex Integration / New Feature
- **Key Findings**:
  1. 既存の`TradeSoA`パターン（`pricer_risk/src/soa/`）をPricingKernelに拡張可能
  2. SIMDアラインメント（64バイト）は`#[repr(align(64))]`ラッパー構造体で実現可能
  3. LSMC実装は既存Rustライブラリ（QuantMath、quantrs）を参考にできる
  4. IndexedMarketパターンと3-stage rocketパターンとの統合が設計の鍵

---

## Research Log

### Topic 1: SIMD 64バイトアラインメント実装方法

- **Context**: Requirement 11で64バイトアラインメントが必要。AVX-512最適化のため。
- **Sources Consulted**:
  - [Rust Forum: Memory alignment for vectorized code](https://users.rust-lang.org/t/memory-alignment-for-vectorized-code/53640)
  - [Rust Forum: Easy way to allocate Vec with 64 byte alignment](https://users.rust-lang.org/t/easy-way-to-allocate-vec-with-64-byte-alignment/95696)
  - [simd_aligned crate](https://docs.rs/simd_aligned)
  - [The state of SIMD in Rust in 2025](https://shnatsel.medium.com/the-state-of-simd-in-rust-in-2025-32c263e5f53d)
- **Findings**:
  - `#[repr(align(64))]`ラッパー構造体でアラインメント可能
  - `simd_aligned`クレートは動的ベクトル用に設計されているが、外部依存となる
  - 自前実装パターン: `AlignedTo64Bytes`構造体 + `Vec::from_raw_parts`
  - `pulp`クレートがAVX-512対応で成熟している（`faer`ライブラリで使用）
- **Implications**:
  - 設計では`AlignedVec<f64>`ラッパー型を定義し、内部でアラインメント保証
  - 外部クレート依存を避け、自前実装を推奨（Enzyme AD互換性のため）

### Topic 2: Data-Oriented Design / SoAパターン

- **Context**: PricingKernelはSoA形式でデータを配置する必要がある
- **Sources Consulted**:
  - [ECS and Data-Oriented Programming](https://prdeving.wordpress.com/2023/12/14/deep-diving-into-entity-component-system-ecs-architecture-and-data-oriented-programming/)
  - [Data Oriented Design is not ECS](https://yoyo-code.com/data-oriented-design-is-not-ecs/)
  - [ECS 2.0 and Data-Oriented Micro-Kernel Architectures](https://www.daydreamsoft.com/blog/ecs-2-0-data-oriented-micro-kernel-architectures-for-massive-persistent-game-worlds)
- **Findings**:
  - SoAはキャッシュ効率を最大100倍改善可能
  - データ連続性がCPUキャッシュ使用率を向上
  - DODは「データを第一級市民として扱う」パラダイム
  - 金融システム特化の事例は少ないが、ゲームエンジンのパターンが適用可能
- **Implications**:
  - `PricingKernel`は純粋なSoA構造として設計
  - 既存の`TradeSoA`（`pricer_risk/src/soa/trade_soa.rs`）パターンを踏襲
  - AoS→SoA変換ロジック（`from_trades()`）をTradeCompilerに実装

### Topic 3: LSMC（Longstaff-Schwartz）実装アプローチ

- **Context**: Requirement 7でCallable/Bermudan商品のBackward Induction実装が必要
- **Sources Consulted**:
  - [Longstaff-Schwartz Original Paper](http://galton.uchicago.edu/~mykland/346W07/Longstaff.pdf)
  - [QuantMath Rust Library](https://github.com/MarcusRainbow/QuantMath)
  - [quantrs Library](https://docs.rs/quantrs/latest/quantrs/)
  - [RustQuant Library](https://github.com/avhz/RustQuant)
- **Findings**:
  - QuantMathは「Longstaff-Schwarz optimisation」を含むと明言
  - quantrsはMonte Carlo SimulationとAmerican optionsをサポート
  - LSMCは「最小二乗回帰のみで実装可能」なシンプルなアルゴリズム
  - Bermudan→Americanは離散時間ステップを細かくすることで近似
- **Implications**:
  - LSMC回帰は`nalgebra`を使用した最小二乗ソルバーで実装可能
  - `CallableKernel`はブロック構造で行使日を管理し、Backward Passで回帰実行
  - 複雑性が高いため、Phase 3（最適化フェーズ）での実装を推奨

### Topic 4: IndexedMarketパターンとの統合

- **Context**: 既存の`IndexedMarket<T>`パターンとの整合性確保
- **Sources Consulted**:
  - `crates/pricer_models/src/market/indexed_market.rs`（内部コード）
  - `crates/infra_master/src/trade/index_requirement.rs`（内部コード）
- **Findings**:
  - `IndexedMarket<T>`は`RateIndex`/`CurrencyPair`をキーとしたHashMapアクセス
  - `TradeIndexRequirements`トレイトが必要なインデックスを宣言
  - 3-stage rocketパターン: Definition → Linking → Execution
- **Implications**:
  - `PricingKernel`は`fwd_index_ids: Vec<u16>`でインデックスIDを保持
  - インデックスID→実データの解決は`KernelContext`（Linking段階）で実施
  - `price_kernel`関数（Execution段階）はインデックスルックアップ不要

---

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| **Option B: 新規コンポーネント作成** | pricer_coreにIR、pricer_modelsにCompiler、pricer_pricingにEngine配置 | A-I-P-S完全遵守、明確な責任境界 | 新規ファイル数が多い | **採用** - アーキテクチャ整合性最優先 |
| Option A: 既存拡張 | pricer_risk/soa/に追加 | 既存パターン再利用 | L4配置はA-I-P-S違反 | 却下 |
| Option C: ハイブリッド | 段階的導入 | リスク分散 | 複数フェーズ調整必要 | 代替案として検討 |

---

## Design Decisions

### Decision 1: PricingKernelの配置場所

- **Context**: PricingKernel構造体をどのクレートに配置するか
- **Alternatives Considered**:
  1. `pricer_core` (L1) - 基盤型として配置
  2. `pricer_models` (L2) - 商品モデルと同階層
  3. `pricer_risk` (L4) - 既存SoAの隣
- **Selected Approach**: `pricer_core/src/ir/` に配置
- **Rationale**:
  - A-I-P-S依存ルール遵守（PricerはServiceに依存されない）
  - L1はL2/L3/L4すべてから参照可能
  - 型定義は最も下位レイヤーに配置すべき
- **Trade-offs**:
  - ✅ 全Pricerクレートから使用可能
  - ❌ pricer_coreの責任範囲が若干拡大
- **Follow-up**: pricer_coreのモジュール構成を`ir/`サブモジュールとして整理

### Decision 2: TradeCompilerの型パラメータ戦略

- **Context**: IndexedMarket<T>は`T: Float`を要求、price_kernelも同様に必要か
- **Alternatives Considered**:
  1. `PricingKernel`も`T: Float`でジェネリック化
  2. `PricingKernel`は`f64`固定、計算時に変換
  3. ハイブリッド: コンパイル結果は`f64`、評価時に`T`変換
- **Selected Approach**: Option 3 ハイブリッド
- **Rationale**:
  - IRはコンパイル済みデータ → `f64`で固定（メモリ効率）
  - 評価関数`price_kernel<T: Float>`で`T`にプロモート
  - Enzyme ADは`f64`→`Dual<f64>`変換に対応
- **Trade-offs**:
  - ✅ IR構造体はシンプル（ジェネリクス汚染なし）
  - ✅ Enzyme AD互換性維持
  - ❌ 評価時に一度だけ型変換が発生
- **Follow-up**: `pricer_core/src/ir/pricing_kernel.rs`で`impl PricingKernel`にヘルパーメソッド追加

### Decision 3: ScriptKernel/CallableKernelのスコープ

- **Context**: 複雑性の高い要件（Req 6, 7）の実装範囲
- **Alternatives Considered**:
  1. 全て同時実装
  2. Phase分割: Phase 1でLinear Products、Phase 2でScript、Phase 3でCallable
  3. ScriptKernelのみ、CallableKernelは将来
- **Selected Approach**: Option 2 Phase分割
- **Rationale**:
  - CallableKernelはLSMC実装が複雑（工数XL）
  - 段階的検証でリスク軽減
  - 早期に価値提供（Linear Productsが大半）
- **Trade-offs**:
  - ✅ 早期リリース可能
  - ✅ フィードバックループ短縮
  - ❌ 全機能完成まで時間がかかる
- **Follow-up**: tasks.mdでPhase分割を明確化

---

## Risks & Mitigations

| Risk | Level | Mitigation |
|------|-------|------------|
| Enzyme ADとVec<i32>（日付）の互換性不明 | Medium | 日付フィールドはAD対象外として分離、微分対象は`f64`のみ |
| LSMC回帰の数値安定性 | Medium | 既存の`nalgebra` QR分解を使用、条件数モニタリング追加 |
| SIMDアラインメントのパフォーマンス効果不明 | Low | ベンチマーク（criterion）で検証、効果がなければ標準Vecに戻す |
| IndexedMarketとの型整合性 | Medium | KernelContextでインデックス解決、price_kernelはインデックスフリー |

---

## References

- [Rust Forum: Memory alignment for vectorized code](https://users.rust-lang.org/t/memory-alignment-for-vectorized-code/53640) — SIMDアラインメント実装パターン
- [simd_aligned crate](https://docs.rs/simd_aligned) — 動的SIMD配列クレート
- [The state of SIMD in Rust in 2025](https://shnatsel.medium.com/the-state-of-simd-in-rust-in-2025-32c263e5f53d) — Rust SIMD現状
- [QuantMath](https://github.com/MarcusRainbow/QuantMath) — Rust金融数学ライブラリ（LSMC参照）
- [quantrs](https://docs.rs/quantrs/latest/quantrs/) — 高速オプション価格計算ライブラリ
- [Longstaff-Schwartz Paper](http://galton.uchicago.edu/~mykland/346W07/Longstaff.pdf) — LSMCオリジナル論文
- [Data-Oriented Design](https://prdeving.wordpress.com/2023/12/14/deep-diving-into-entity-component-system-ecs-architecture-and-data-oriented-programming/) — ECS/DOD解説
