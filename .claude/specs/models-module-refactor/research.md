# Research & Design Decisions: models-module-refactor

## Summary
- **Feature**: `models-module-refactor`
- **Discovery Scope**: Extension（既存システムのリファクタリング）
- **Key Findings**:
  - `models/sabr.rs` の `StochasticModel` 実装は存在するが、外部から直接使用されていない
  - `analytical/distributions.rs` は `pricer_core` からの単純な re-export
  - `market/volcube` は独自の `SabrParams` を持ち、`SABRModel::implied_vol()` を間接参照

## Research Log

### SABRModel の使用状況調査
- **Context**: SABR SDE 部分（`evolve_step`）の使用有無を確認
- **Sources Consulted**:
  - `crates/pricer_models/src/models/sabr.rs` (L1022-1114)
  - `crates/pricer_models/src/models/model_enum.rs`
  - `crates/pricer_models/src/market/volcube/`
  - `crates/pricer_models/src/market/calibration/sabr.rs`
- **Findings**:
  - `StochasticModel` 実装は完全（`evolve_step`, `initial_state`, `brownian_dim`）
  - `model_enum.rs` で `SABRModel` を enum variant として参照
  - **直接的な MC シミュレーション使用箇所なし**
  - VolCube/キャリブレーションは `implied_vol()` のみを使用
- **Implications**: SABR SDE 実装は削除可能。`model_enum.rs` から SABR variant を除去する必要あり

### analytical/distributions.rs の構造
- **Context**: 重複排除の対象範囲を確認
- **Sources Consulted**: `crates/pricer_models/src/analytical/distributions.rs`
- **Findings**:
  ```rust
  // 既存内容（全3行）
  pub use pricer_core::math::distributions::{norm_cdf, norm_inv_cdf, norm_pdf};
  ```
  - 純粋な re-export のみ、独自実装なし
  - `black_scholes.rs`, `bachelier.rs` がこの re-export を使用
- **Implications**: ファイル削除後、各使用箇所で `pricer_core::math::distributions` を直接参照

### market/volcube の SABR 参照構造
- **Context**: リファクタリングが VolCube に与える影響を評価
- **Sources Consulted**:
  - `crates/pricer_models/src/market/volcube/types.rs`
  - `crates/pricer_models/src/market/volcube/sabr_surface.rs`
- **Findings**:
  - `volcube/types.rs` には `SabrParams<T>` の定義が**ない**（`VolInstrument` 等のみ）
  - `sabr_surface.rs` は `SabrParams` を import しているが、独自定義ではなく外部参照
  - SABR パラメータ補間は `BilinearInterpolator` で行われ、`implied_vol` 計算は別箇所
- **Implications**: `formulas/sabr_implied_vol.rs` 作成後、VolCube は新モジュールを参照可能

### 後方互換性の影響範囲
- **Context**: `#[deprecated]` re-export の設計
- **Sources Consulted**:
  - `pricer_models::models::SABRParams` の使用箇所（Grep 結果）
  - `pricer_models::analytical::*` の使用箇所
- **Findings**:
  - 公開 API: `SABRParams`, `SABRModel`, `SABRError`, `BlackScholes`, `Bachelier`, `GarmanKohlhagen`
  - ドキュメント内の use 文（doc test）が最多（約15箇所）
  - 外部クレートからの直接参照: `pricer_pricing` の integration tests
- **Implications**:
  - `lib.rs` で deprecated re-export を追加
  - doc test の更新が必要

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| **Flat Rename** (採用) | `models/` → `stochastic/`, `analytical/` → `formulas/` | 最小限の変更、明確な命名 | 外部依存の破壊 | deprecated re-export で軽減 |
| Nested Submodules | `formulas/pricing/`, `formulas/implied_vol/` | より細かい分類 | 過剰な階層化 | ユーザー否定 |
| 完全統合 | 全てを `formulas/` に統合 | シンプル | 確率過程の概念が失われる | 不採用 |

## Design Decisions

### Decision: ディレクトリ名を `stochastic/` と `formulas/` に変更
- **Context**: `models/` と `analytical/` という名称が曖昧
- **Alternatives Considered**:
  1. `models/` → `processes/` — やや長い
  2. `models/` → `mc_models/` — MC に限定しすぎ
  3. `models/` → `stochastic/` — 確率過程を正確に表現
- **Selected Approach**: `stochastic/` と `formulas/`
- **Rationale**:
  - "stochastic" は確率過程モデルを正確に表す数学用語
  - "formulas" は閉形式解・近似公式を直感的に表現
- **Trade-offs**:
  - ✅ 概念的に正確
  - ❌ import パスの変更が必要
- **Follow-up**: deprecated re-export で移行期間を確保

### Decision: SABR SDE 実装を削除
- **Context**: `StochasticModel for SABRModel` の実装存在
- **Alternatives Considered**:
  1. 実装を残して `stochastic/sabr_process.rs` に移動
  2. 実装を完全削除
- **Selected Approach**: 完全削除
- **Rationale**:
  - ギャップ分析で「未使用」と確認済み
  - MC シミュレーションでの SABR 使用実績なし
  - コードの複雑性削減
- **Trade-offs**:
  - ✅ コード簡素化
  - ❌ 将来 SABR MC が必要になった場合は再実装
- **Follow-up**: `model_enum.rs` から SABR variant を除去

### Decision: sabr_implied_vol.rs の API 設計
- **Context**: 既存 `SABRModel` から Hagan 公式部分を抽出
- **Alternatives Considered**:
  1. `SABRModel` をそのまま移動（名前変更なし）
  2. `SabrImpliedVol` として新規作成
  3. `SabrHagan` として関数ベースで提供
- **Selected Approach**: `SabrImpliedVol<T>` 構造体 + 既存 API 維持
- **Rationale**:
  - `SABRParams<T>` と `SABRModel::implied_vol()` の互換性維持
  - 構造体ベースで状態（パラメータ）を保持
- **Trade-offs**:
  - ✅ 後方互換性が高い
  - ❌ 一時的に2つの SABR 関連型が存在
- **Follow-up**:
  - `SABRParams` → `SabrParams` に名前変更検討（命名規則統一）
  - deprecated `SABRModel` を `formulas::SabrImpliedVol` から re-export

## Risks & Mitigations
- **Risk 1**: 外部クレートの import 破壊 → deprecated re-export + 移行ガイド
- **Risk 2**: VolCube との整合性 → `SabrParams` の統一的な定義場所を確立
- **Risk 3**: model_enum の SABR 削除による影響 → feature flag "equity" の確認

## References
- [Hagan et al. (2002)](https://www.researchgate.net/publication/235622441) — SABR implied vol 公式の原典
- `pricer_core::math::distributions` — 正規分布関数の canonical 実装
- `.claude/steering/structure.md` — A-I-P-S アーキテクチャ定義
