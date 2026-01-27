# Research & Design Decisions: remove-dual-ad

---
**Purpose**: Dual Number（num-dual）方式の自動微分削除に関する調査結果と設計判断を記録
---

## Summary
- **Feature**: `remove-dual-ad`
- **Discovery Scope**: Extension（既存システムの簡素化リファクタリング）
- **Key Findings**:
  - `solve_ad`と`compute_vega_forward_ad`は定義のみで実際の呼び出し箇所なし（影響ゼロ）
  - `GreeksMode::NumDual`はテストコードのみで使用（本番コード影響なし）
  - ドキュメント参照は約70ファイルだが、docコメントのみで実装影響なし

## Research Log

### solve_ad関数の使用状況調査
- **Context**: Gap Analysisで`solve_ad`がPublic APIであり、削除がBreaking changeとなる可能性を懸念
- **Sources Consulted**: `grep -r "solve_ad"` でコードベース全体を検索
- **Findings**:
  - `pricer_core/src/math/solvers/newton_raphson.rs`に定義のみ存在
  - demo, service_python, 他モジュールでの呼び出し箇所なし
  - 関数は`#[cfg(feature = "num-dual-mode")]`で条件コンパイル
- **Implications**: Breaking changeの影響は事実上ゼロ。ドキュメント告知のみで削除可能。

### compute_vega_forward_ad関数の使用状況調査
- **Context**: VolCube Vega計算でforward mode ADを使用
- **Sources Consulted**: `grep -r "compute_vega_forward_ad"` でコードベース全体を検索
- **Findings**:
  - `pricer_models/src/market/volcube/vega.rs`に定義のみ存在
  - 他モジュールからの呼び出し箇所なし
  - Vegaは`compute_vega_finite_difference`が代替として存在
- **Implications**: 削除しても機能影響なし。BumpRevalueで代替可能。

### GreeksMode::NumDual使用箇所調査
- **Context**: GreeksModeの列挙型バリアントとして存在、削除による影響確認
- **Sources Consulted**: `grep -r "GreeksMode::NumDual"` でコードベース全体を検索
- **Findings**:
  - 使用箇所4ファイル（すべてテストコード）
    - `pricer_risk/src/scenarios/greeks_by_factor.rs` (テスト)
    - `pricer_risk/benches/risk.rs` (ベンチマーク)
    - `pricer_risk/src/greeks/tests.rs` (テスト)
    - `pricer_pricing/src/generic_pricer/config.rs` (テスト)
  - 本番コードでの使用なし
- **Implications**: テストコード修正のみで削除可能。本番影響なし。

### Enzyme未環境での代替策
- **Context**: Enzyme未インストール環境でのGreeks計算方法
- **Sources Consulted**: `pricer_risk/src/enzyme/fallback.rs`, `pricer_risk/src/greeks/config.rs`
- **Findings**:
  - `GreeksMode::BumpRevalue`がデフォルトとして存在
  - Enzyme未環境では自動的にfinite difference fallbackを使用
  - `enzyme/fallback.rs`モジュールが代替計算を提供
- **Implications**: num-dual削除後もBumpRevalueで全機能カバー可能

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| 完全削除 | num-dual関連コードを全削除 | コード簡素化、依存削減、保守コスト減 | Breaking change（ただし影響ゼロ確認済） | **採用** |
| 非推奨化 | `#[deprecated]`で段階的移行 | 後方互換性、移行猶予 | 技術的負債継続、2バックエンド保守 | 不採用（使用箇所なしのため不要） |
| Hybrid | solve_adをEnzyme wrapperで再実装 | API互換性維持 | 複雑度増加、Enzyme必須化 | 不採用（使用箇所なしのため不要） |

## Design Decisions

### Decision: 完全削除アプローチの採用
- **Context**: Dual Number方式のADコードをどの程度削除するか
- **Alternatives Considered**:
  1. 完全削除 — 全てのnum-dual関連コードを削除
  2. 非推奨化 — `#[deprecated]`で段階的移行
  3. Hybrid — 部分的にEnzyme wrapperで再実装
- **Selected Approach**: 完全削除
- **Rationale**:
  - `solve_ad`、`compute_vega_forward_ad`は使用箇所なし
  - `GreeksMode::NumDual`はテストコードのみ
  - 非推奨化の猶予期間は不要
- **Trade-offs**:
  - ✅ コードベース大幅簡素化
  - ✅ 依存関係1つ削減（num-dual）
  - ✅ default featuresからnum-dual-mode削除でビルド高速化
  - ❌ 将来的にnum-dual再導入時は再実装必要（可能性低）
- **Follow-up**: CHANGELOGにBreaking change記載（影響なしでも形式的に）

### Decision: ドキュメント更新戦略
- **Context**: 約70ファイルのdocコメントで`Dual64`への言及あり
- **Alternatives Considered**:
  1. 全削除 — `Dual64`言及を削除
  2. 置換 — `f64`のみに置換
  3. 維持 — AD互換性言及として残す（誤解を招く）
- **Selected Approach**: 置換（`Dual64` → 削除、`f64`のみ記載）
- **Rationale**: 型パラメータTはf64で十分であり、将来のEnzyme AD互換は別の説明が必要
- **Trade-offs**:
  - ✅ ドキュメントの正確性向上
  - ❌ 一括置換の作業コスト（低い）
- **Follow-up**: sed/grepで一括置換、CI通過確認

### Decision: GreeksMode列挙型の更新
- **Context**: `GreeksMode::NumDual`バリアントの扱い
- **Alternatives Considered**:
  1. 即時削除 — バリアントを削除
  2. 非推奨化 — `#[deprecated]`追加後、次版で削除
- **Selected Approach**: 即時削除
- **Rationale**: テストコードのみで使用されており、非推奨化の猶予は不要
- **Trade-offs**:
  - ✅ クリーンな列挙型
  - ❌ テストコード修正必要（4ファイル）
- **Follow-up**: テストコードを`GreeksMode::BumpRevalue`に置換

## Risks & Mitigations

| リスク | 緩和策 |
|--------|--------|
| 見落としimport/use | CI全ターゲットでビルド・テスト実行 |
| 外部ユーザー影響 | CHANGELOGでBreaking change明示（実影響なし） |
| ドキュメント整合性 | 一括sed置換後、CI docsビルド確認 |
| Enzyme未環境 | BumpRevalue fallback維持（既存機能） |

## References

- [num-dual crate](https://crates.io/crates/num-dual) — Dual number library
- [Enzyme AD](https://enzyme.mit.edu/) — LLVM-level automatic differentiation
- Steering: `tech.md` — AD Backend設計方針
- Steering: `structure.md` — pricer_core types構造

---
_Generated: 2026-01-27_
