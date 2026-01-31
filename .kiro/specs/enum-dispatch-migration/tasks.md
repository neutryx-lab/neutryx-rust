# Implementation Plan

## タスク概要

本実装計画は、`enum_dispatch` クレートの導入により、Neutryx デリバティブ価格計算ライブラリの Enum-Trait ボイラープレートを排除する。設計ドキュメントの移行戦略に従い、7つの主要タスクで4つの Enum（CurveEnum、FxCurveEnum、WorkspaceEnum、PathPayoffType）を移行する。

**除外**: `StochasticModelEnum`（関連型制約のため技術的に不可能）

---

## Tasks

- [x] 1. ワークスペースへの enum_dispatch 依存関係追加
- [x] 1.1 Cargo.toml に enum_dispatch クレートを設定する
  - ワークスペースルートの `[workspace.dependencies]` セクションに `enum_dispatch` を追加
  - バージョン `0.3` 以上を指定
  - 全クレートでコンパイル成功を確認
  - _Requirements: 1.1, 1.2, 1.3, 1.4_

- [x] 2. pricer_models における CurveEnum の移行
- [x] 2.1 (P) YieldCurve トレイトに enum_dispatch 属性を適用する
  - `pricer_models` クレートに `enum_dispatch` 依存を追加（workspace 継承）
  - `YieldCurve<T>` トレイト定義に `#[enum_dispatch]` マクロを付与
  - インポート文 `use enum_dispatch::enum_dispatch;` を追加
  - _Requirements: 4.1_

- [x] 2.2 CurveEnum に enum_dispatch 属性を適用し手動実装を削除する
  - `CurveEnum<T>` に `#[enum_dispatch(YieldCurve<T>)]` 属性を付与
  - 手動 `impl<T: Float> YieldCurve<T> for CurveEnum<T>` ブロックを完全に削除
  - 各バリアント型（FlatCurve、BootstrappedCurve）が YieldCurve を実装していることを確認
  - 既存のブートストラップテストが全てパスすることを検証
  - _Requirements: 4.2, 4.3, 4.4, 4.5_

- [x] 3. pricer_models における FxCurveEnum の移行
- [x] 3.1 (P) FxCurve トレイトに enum_dispatch 属性を適用する
  - `FxCurve<T>` トレイト定義に `#[enum_dispatch]` マクロを付与
  - トレイトメソッド（forward、spot、currency_pair）の互換性を確認
  - _Requirements: 4.1_

- [x] 3.2 FxCurveEnum に enum_dispatch 属性を適用し手動実装を削除する
  - `FxCurveEnum<T>` に `#[enum_dispatch(FxCurve<T>)]` 属性を付与
  - 手動 `impl<T: Float> FxCurve<T> for FxCurveEnum<T>` ブロックを削除
  - IrpFxCurve を含む全バリアントの正常動作を検証
  - _Requirements: 4.2, 4.3, 4.4, 4.5_

- [x] 4. pricer_pricing における WorkspaceEnum の移行
- [x] 4.1 PathWorkspaceTrait に enum_dispatch 属性を適用する
  - `pricer_pricing` クレートに `enum_dispatch` 依存を追加（workspace 継承）
  - `PathWorkspaceTrait` に `#[enum_dispatch]` マクロを付与
  - 10以上のメソッドが正しく転送されることを確認
  - _Requirements: 7.4_

- [x] 4.2 WorkspaceEnum に enum_dispatch 属性を適用し手動実装を削除する
  - `WorkspaceEnum` に `#[enum_dispatch(PathWorkspaceTrait)]` 属性を付与
  - 70行以上の手動 `impl PathWorkspaceTrait for WorkspaceEnum` を削除
  - PathFirst と TimeStepFirst レイアウトの切り替えが正常に動作することを検証
  - inherent methods（ensure_capacity、reset、reset_fast）は維持
  - _Requirements: 7.4, 7.5_

- [x] 5. pricer_pricing における PathPayoffType の移行
- [x] 5.1 PathDependentPayoff トレイトを定義し enum_dispatch 属性を適用する
  - 既存の inherent methods（compute、required_observations、smoothing_epsilon）をトレイトメソッドとして抽出
  - 新規トレイト `PathDependentPayoff<T>` を定義し `#[enum_dispatch]` を付与
  - Send + Sync 境界を維持
  - _Requirements: 5.1_

- [x] 5.2 PathPayoffType に enum_dispatch 属性を適用し inherent methods を削除する
  - `PathPayoffType<T>` に `#[enum_dispatch(PathDependentPayoff<T>)]` 属性を付与
  - 各バリアント型（AsianArithmeticPayoff、BarrierPayoff 等）にトレイト実装を追加
  - 既存の inherent compute/required_observations/smoothing_epsilon を削除
  - is_asian/is_barrier/is_lookback は inherent methods として維持
  - Asian/Barrier/Lookback ペイオフ計算結果が移行前と同一であることを検証
  - _Requirements: 5.2, 5.3, 5.4_

- [x] 6. Enzyme AD 互換性の検証
- [x] 6.1 nightly ビルドでのコンパイル検証を実施する
  - `cargo +nightly build -p pricer_pricing --features all` が成功することを確認
  - enum_dispatch 生成コードが Enzyme LLVM プラグインと互換であることを検証
  - コンパイルエラーが発生した場合、該当 Enum を移行対象から除外し手動実装を復元
  - _Requirements: 6.1, 6.3_

- [x] 6.2 AD 微分計算の正確性を検証する
  - PathPayoffType を使用した Monte Carlo シミュレーションで Greeks を計算
  - bump-and-revalue との比較テストを実施
  - 許容誤差範囲内（相対誤差 1e-6 以下）であることを確認
  - 性能ベンチマークで移行前後の速度劣化がないことを検証
  - _Requirements: 6.2, 6.4_

- [x] 7. コード品質確認とクリーンアップ
- [x] 7.1 静的解析とフォーマット検証を実施する
  - `cargo clippy --workspace -- -D warnings` が警告なしでパス
  - `cargo fmt --all -- --check` がフォーマットエラーなしでパス
  - 移行対象 Enum の手動 match トレイト転送パターンが完全に排除されていることを確認
  - _Requirements: 7.1, 7.2, 7.5_

- [x] 7.2 既存 API の後方互換性を検証する
  - 公開 API の関数シグネチャ、型定義、エクスポートに変更がないことを確認
  - `cargo test --workspace` で全テストスイートがパス
  - セマンティックバージョニングにおいて破壊的変更が発生していないことを確認
  - _Requirements: 8.1, 8.2, 8.3, 8.4_

---

## Requirements Coverage

| Requirement | Task Coverage |
|-------------|---------------|
| 1.1, 1.2, 1.3, 1.4 | 1.1 |
| 2.1, 2.2, 2.3 | 設計時に完了（対象 Enum 識別済み） |
| 3.1, 3.2, 3.3, 3.4, 3.5 | **除外**（関連型制約） |
| 4.1 | 2.1, 3.1 |
| 4.2, 4.3, 4.4, 4.5 | 2.2, 3.2 |
| 5.1 | 5.1 |
| 5.2, 5.3, 5.4 | 5.2 |
| 6.1, 6.3 | 6.1 |
| 6.2, 6.4 | 6.2 |
| 7.1, 7.2, 7.5 | 7.1 |
| 7.3 | 設計内で完了（ドキュメントコメント維持） |
| 7.4 | 4.1, 4.2 |
| 8.1, 8.2, 8.3, 8.4 | 7.2 |

**Requirement 2 (対象 Enum 識別)**: Gap Analysis と設計フェーズで完了済み。実装タスクでは対象 Enum が既に特定されている。

**Requirement 3 (StochasticModelEnum)**: 関連型（State, Params）により enum_dispatch が適用不可。現行 inherent methods 実装を維持。
