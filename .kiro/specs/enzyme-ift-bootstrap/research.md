# Research & Design Decisions: enzyme-ift-bootstrap

## Summary

- **Feature**: `enzyme-ift-bootstrap`
- **Discovery Scope**: Complex Integration / Extension
- **Key Findings**:
  1. nalgebra-sparse が nalgebra との自然な統合を提供、sprs は単体ライブラリとして成熟
  2. Enzyme AD の Rust 統合は GSoC 2025 で安定化作業進行中、TypeTrees による型情報最適化
  3. GMRES 実装は gmres crate および scirs2-sparse で利用可能
  4. `LinearSolveStrategy<T>` trait は拡張可能な設計、新規 Strategy 追加が自然

## Research Log

### 疎行列ライブラリ選定 (nalgebra-sparse vs sprs)

- **Context**: Requirement 4 で LinearSolveStrategy に疎行列サポートを追加する必要あり
- **Sources Consulted**:
  - [nalgebra-sparse — Lib.rs](https://lib.rs/crates/nalgebra-sparse)
  - [nalgebra-sparse docs.rs](https://docs.rs/nalgebra-sparse/0.4.0/nalgebra_sparse/)
  - [sprs — crates.io](https://crates.io/crates/sprs)
  - [sprs GitHub](https://github.com/sparsemat/sprs)

- **Findings**:
  | 観点 | nalgebra-sparse | sprs |
  |------|-----------------|------|
  | nalgebra 統合 | ネイティブ統合 | 別途変換必要 |
  | フォーマット | CSR, CSC, COO | CSR, CSC |
  | API 成熟度 | Idiomatic Rust | Work-in-progress |
  | 並列処理 | 目標として記載 | 言及なし |
  | MSRV | nalgebra と同等 | 1.64 |

- **Implications**:
  - nalgebra-sparse を推奨（既存 nalgebra インフラとの整合性）
  - `DMatrix<T>` との相互変換が容易
  - feature-gated 導入が可能（`sparse` feature）

### Enzyme AD Rust 統合

- **Context**: Requirement 1 で JacobianMethod::AutomaticDifferentiation の完全実装が必要
- **Sources Consulted**:
  - [Enzyme AD 公式サイト](https://enzyme.mit.edu/)
  - [Enzyme GitHub](https://github.com/EnzymeAD/Enzyme)
  - [GSoC 2025 Enzyme Rust Blog](https://blog.karanjanthe.me/posts/enzyme-autodiff-rust-gsoc/)
  - [std::intrinsics::autodiff docs](https://doc.rust-lang.org/nightly/std/intrinsics/fn.autodiff.html)

- **Findings**:
  - Enzyme は LLVM IR レベルで動作、言語非依存の AD
  - Rust では型解析フェーズがボトルネック（TypeTrees で改善中）
  - GSoC 2025 で安定化作業進行中
  - `#[autodiff]` マクロによる宣言的 AD が可能
  - nightly-2025-01-15 以降で利用可能

- **Implications**:
  - 既存の `#[autodiff]` パターン（kernel.rs）を踏襲
  - CalibrationProblem にも同様のアプローチ適用可能
  - feature-gated 実装で stable 互換性維持

### GMRES 反復解法

- **Context**: Requirement 4.5 で反復解法（GMRES）による疎行列解法が必要
- **Sources Consulted**:
  - [gmres crate](https://crates.io/crates/gmres)
  - [scirs2-sparse crate](https://crates.io/crates/scirs2-sparse)
  - [gmres GitHub](https://github.com/rlado/GMRES)

- **Findings**:
  - `gmres` crate は単独の GMRES 実装
  - `scirs2-sparse` は包括的な反復解法スイート（CG, BiCG, GMRES, QMR）
  - プリコンディショナー対応あり
  - CSR/CSC フォーマット対応

- **Implications**:
  - scirs2-sparse が包括的だが、nalgebra-sparse との統合検証必要
  - 初期実装は直接解法（Sparse LU）、反復解法は将来拡張として

### 既存コードパターン分析

- **Context**: 設計の一貫性を確保するため、既存パターンを分析
- **Sources Consulted**:
  - `pricer_core/src/math/linalg/strategy.rs`
  - `pricer_models/src/builder/problem.rs`
  - `pricer_models/src/builder/curve/global.rs`
  - `pricer_risk/src/greeks/ad/shadow.rs`

- **Findings**:
  1. **LinearSolveStrategy パターン**:
     ```rust
     pub trait LinearSolveStrategy<T: RealField + Copy + Float>: Clone + Default {
         fn decompose(&mut self, matrix: &DMatrix<T>) -> Result<(), LinearAlgebraError>;
         fn solve(&self, b: &[T]) -> Result<Vec<T>, LinearAlgebraError>;
         fn inverse(&self) -> Result<DMatrix<T>, LinearAlgebraError>;
     }
     ```
     - 新規 Strategy 追加が自然な拡張ポイント
     - `validate_structure` でオプション検証可能

  2. **JacobianMethod enum**:
     ```rust
     pub enum JacobianMethod {
         Analytical,
         #[default]
         FiniteDifference,
         CentralDifference,
         #[cfg(feature = "enzyme-ad")]
         AutomaticDifferentiation,
     }
     ```
     - 既に feature-gated AD variant が存在
     - 現在はスタブ実装（finite diff にフォールバック）

  3. **GlobalBootstrapResult**:
     - `jacobian_inverse: Option<DMatrix<T>>` が既に存在
     - IFT 感度用の J⁻¹ キャッシュは設計済み
     - `ift_sensitivity` メソッドの追加のみ必要

  4. **Shadow trait**:
     - `Clone` bound + `zero_out()` + `create_shadow()` パターン
     - SimpleYieldCurve, SimpleVolSurface 等に実装済み
     - GlobalBootstrapResult への実装が必要

- **Implications**:
  - 既存パターンに沿った自然な拡張が可能
  - 新規ファイル作成は sparse モジュールのみ
  - API 変更は後方互換性を維持

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Option A: Extend | 既存ファイル直接拡張 | 最小ファイル追加、学習コスト低 | 肥大化リスク、後方互換性管理 | 小規模変更向け |
| Option B: New | 専用モジュール新設 | 責務分離、単体テスト容易 | コード重複、統合ポイント設計 | 大規模新機能向け |
| **Option C: Hybrid** | 疎行列は新規、IFT/AAD は拡張 | 段階的デリバリー、リスク分散 | 複数フェーズ調整 | **推奨** |

## Design Decisions

### Decision: 疎行列ライブラリの選定

- **Context**: LinearSolveStrategy に疎行列サポートを追加
- **Alternatives Considered**:
  1. nalgebra-sparse — nalgebra ネイティブ統合
  2. sprs — 成熟した単体ライブラリ
  3. 自前実装 — 完全制御
- **Selected Approach**: nalgebra-sparse
- **Rationale**: 既存 nalgebra インフラとの型互換性、pricer_core の設計思想との整合性
- **Trade-offs**:
  - ✅ DMatrix<T> との相互変換が容易
  - ✅ RealField 制約との互換性
  - ❌ sprs より機能が限定的
- **Follow-up**: GMRES 統合は Phase 2 として分離

### Decision: IFT API の配置

- **Context**: IFT 感度抽出メソッドの配置場所
- **Alternatives Considered**:
  1. GlobalBootstrapResult に ift_sensitivity メソッド追加
  2. ImplicitSolver を拡張
  3. 新規 IftSensitivityCalculator 作成
- **Selected Approach**: GlobalBootstrapResult にメソッド追加
- **Rationale**:
  - J⁻¹ は GlobalBootstrapResult に既にキャッシュ
  - 凝集度が高い（データと操作の近接）
  - ImplicitSolver は pricer_risk 層、GlobalBootstrapResult は pricer_models 層
- **Trade-offs**:
  - ✅ 既存構造の自然な拡張
  - ✅ 使用側のコード変更最小
  - ❌ GlobalBootstrapResult の責務増加

### Decision: Feature Gate 粒度

- **Context**: 新機能の feature flag 設計
- **Alternatives Considered**:
  1. 単一 `enzyme-ad` feature に統合
  2. `enzyme-ad` + `sparse` 分離
  3. 細粒度（enzyme-ad, sparse, ift, stability）
- **Selected Approach**: `enzyme-ad` + `sparse` 分離
- **Rationale**:
  - 疎行列は Enzyme なしでも有用
  - 独立コンパイル・テスト可能
  - feature propagation の複雑化回避
- **Trade-offs**:
  - ✅ 柔軟な組み合わせ
  - ✅ CI 並列化可能
  - ❌ Cargo.toml 管理増加

### Decision: 数値安定性アプローチ

- **Context**: 条件数モニタリングと正則化
- **Alternatives Considered**:
  1. SVD による厳密条件数計算
  2. 1-norm/∞-norm 推定
  3. 行列対角線ベース簡易推定
- **Selected Approach**: 1-norm 推定 + Tikhonov 正則化
- **Rationale**:
  - SVD は O(n³) で大規模行列に非効率
  - 1-norm 推定は既存 GlobalBootstrapper.estimate_condition_number と整合
  - Tikhonov は LM スタイル damping と互換
- **Trade-offs**:
  - ✅ 計算効率
  - ✅ 既存コードとの整合性
  - ❌ 厳密性の低下

## Risks & Mitigations

1. **Enzyme LLVM バージョン依存** — Docker/CI で LLVM 18 固定、stable フォールバック提供
2. **疎行列パフォーマンス不確実性** — ベンチマーク駆動開発、70% スパース閾値は調整可能に
3. **条件数推定精度** — 警告のみ（エラーではない）、max_condition_number はユーザー設定可能
4. **後方互換性** — LinearSolveStrategy trait に optional メソッドを追加（デフォルト実装付き）
5. **クロスプラットフォーム** — Windows Enzyme サポートは Research Needed、CI でマルチプラットフォームテスト

## References

- [nalgebra-sparse documentation](https://docs.rs/nalgebra-sparse/0.4.0/nalgebra_sparse/) — 疎行列フォーマットと API
- [Enzyme AD](https://enzyme.mit.edu/) — 公式サイト、概念とアーキテクチャ
- [GSoC 2025 Enzyme Rust](https://blog.karanjanthe.me/posts/enzyme-autodiff-rust-gsoc/) — Rust 統合の最新状況
- [gmres crate](https://crates.io/crates/gmres) — Rust GMRES 実装
- [scirs2-sparse](https://crates.io/crates/scirs2-sparse) — 包括的疎行列線形代数
