# Research & Design Decisions

## Summary
- **Feature**: `greeks-calculation`
- **Discovery Scope**: Extension（既存システムの拡張）
- **Key Findings**:
  - 既存のMonteCarloPricerに一次Greeks（Delta, Gamma, Vega, Theta, Rho）のbump-and-revalue実装済み
  - 手動タンジェント伝播によるforward-mode AD（Delta専用）が実装済み
  - 二次Greeks（Vanna, Volga）およびEnzyme AAD統合は未実装

## Research Log

### 既存Greeks実装の分析
- **Context**: 現在のpricer_kernelにおけるGreeks計算の実装状況調査
- **Sources Consulted**: `crates/pricer_kernel/src/mc/pricer.rs`
- **Findings**:
  - `Greek` enum: Delta, Gamma, Vega, Theta, Rhoを定義
  - `PricingResult`: price, std_error, delta, gamma, vega, theta, rhoを持つf64固定型
  - `price_with_greeks()`: 選択的Greeks計算をサポート
  - `price_with_delta_ad()`: 手動タンジェント伝播（forward-mode）を実装
  - パス依存オプション用Greeks: `price_path_dependent_with_greeks()`実装済み
- **Implications**:
  - 拡張アプローチ採用（新規作成不要）
  - `GreeksResult<T>`ジェネリック型の導入が必要
  - Vanna/Volgaの追加が必要

### Bump幅設定の現状
- **Context**: 有限差分法のバンプ幅設定調査
- **Sources Consulted**: `pricer.rs` compute_delta, compute_gamma等
- **Findings**:
  - Delta: `0.01 * spot` または最小 `0.01`（相対1%）
  - Vega: 絶対値 `0.01`（1%vol）
  - Theta: `1/252`（1日）
  - Rho: 絶対値 `0.01`（1%rate）
  - バンプ幅はハードコード、設定不可
- **Implications**:
  - `GreeksConfig`構造体でバンプ幅を設定可能にする
  - 既存デフォルト値を維持しつつカスタマイズ可能に

### Enzyme AD統合計画
- **Context**: Phase 4でのEnzyme統合準備状況
- **Sources Consulted**: steering/tech.md, steering/structure.md
- **Findings**:
  - Enzyme: LLVM 18プラグイン、nightly-2025-01-15必須
  - pricer_kernel（L3）のみnightly使用
  - `#[autodiff]`マクロ使用予定
  - feature flag: `enzyme-mode`
- **Implications**:
  - GreeksEngineはfeature flagで計算モード切り替え
  - AADとbump-and-revalueの両方をサポート
  - Enzyme統合はPhase 4で実装

### Path Dependent Options Greeks
- **Context**: パス依存オプションのGreeks計算調査
- **Sources Consulted**: `path_dependent/payoff.rs`, `path_dependent/observer.rs`
- **Findings**:
  - `PathObserver<T>`: ストリーミング統計（平均、最大、最小）
  - `PathDependentPayoff<T>` trait: compute, required_observations, smoothing_epsilon
  - `PathPayoffType<f64>`: 静的ディスパッチenum
  - smooth近似: barrier関数にsmooth_indicatorを使用
- **Implications**:
  - PathObserverのジェネリック型を活用
  - smooth近似によりAD互換性を維持

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| 既存Pricer拡張 | MonteCarloPricerに直接機能追加 | 互換性維持、既存テスト活用 | pricer.rsの肥大化 | 現状のアプローチ |
| GreeksEngine分離 | 独立したGreeks計算エンジン | 関心分離、テスト容易性 | 追加の抽象化レイヤー | 推奨アプローチ |
| Trait抽象化 | GreeksCalculator trait | 柔軟性、複数実装 | 複雑性増加 | 将来的な拡張用 |

**選択**: GreeksEngine分離 + 既存Pricer統合

## Design Decisions

### Decision: GreeksResult<T>ジェネリック構造体の導入
- **Context**: 現在のPricingResultはf64固定でAD非互換
- **Alternatives Considered**:
  1. 既存PricingResult維持 — AD非互換
  2. GreeksResult<T>新規追加 — 柔軟性向上
- **Selected Approach**: GreeksResult<T>新規追加、PricingResultをGreeksResult<f64>のtype aliasに
- **Rationale**: num-dual/Enzyme両モードでの計算をサポート
- **Trade-offs**: 追加の型定義 vs AD互換性
- **Follow-up**: 既存テストの互換性確認

### Decision: GreeksConfig構造体によるバンプ幅設定
- **Context**: 現在のバンプ幅がハードコード
- **Alternatives Considered**:
  1. グローバル定数 — 柔軟性なし
  2. 関数パラメータ — API複雑化
  3. Config構造体 — 一貫したAPI
- **Selected Approach**: GreeksConfig構造体（Builder pattern）
- **Rationale**: MonteCarloConfigと同様のパターンで一貫性維持
- **Trade-offs**: 追加の構造体 vs 設定可能性
- **Follow-up**: デフォルト値の妥当性検証

### Decision: 計算モード切り替え（Bump vs AAD）
- **Context**: 複数の計算手法をサポート
- **Alternatives Considered**:
  1. 別メソッド — 明示的だが冗長
  2. Enum引数 — 柔軟だが実行時判断
  3. Feature flag — コンパイル時最適化
- **Selected Approach**: Feature flag + Enum引数のハイブリッド
- **Rationale**: コンパイル時最適化（enzyme-mode）と実行時柔軟性の両立
- **Trade-offs**: 条件分岐コード vs 最適化
- **Follow-up**: feature flagの境界テスト

## Risks & Mitigations
- **Risk 1**: Enzyme統合の複雑性 — 段階的実装（Phase 4）で軽減
- **Risk 2**: 二次Greeks数値安定性 — smooth近似の適切なepsilon選択
- **Risk 3**: パフォーマンス劣化 — ベンチマーク追加、criterionによる回帰テスト

## References
- [Enzyme AD](https://enzyme.mit.edu/) — LLVM自動微分
- [num-dual](https://docs.rs/num-dual/) — Rust dual numbers
- [pricer_kernel/mc/pricer.rs](crates/pricer_kernel/src/mc/pricer.rs) — 既存実装
