# Implementation Plan

## Task Overview

Phase 1の実装タスク: Layer 1 (pricer_core) の数学的基盤を構築します。スムージング関数、Dual数型統合、基本トレイト、時間型を実装し、全ての機能が安定版Rustでビルド可能でEnzymeとnum-dualの双方で動作することを検証します。

## Tasks

- [x] 1. プロジェクト構造とCargo設定を構築する
- [x] 1.1 (P) pricer_coreクレートのCargo.toml設定を作成する
  - Rust Edition 2021を指定し、ワークスペースメンバーとして登録
  - 依存関係としてnum-traits 0.2、num-dual 0.9、chrono 0.4を追加
  - dev-dependenciesとしてproptest 1.0、approx 0.5、criterion 0.5を追加
  - 他のpricer_*クレートへの依存を持たないことを確認
  - _Requirements: 2, 4, 8, 9_

- [x] 1.2 (P) モジュール構造の骨格を作成する
  - `lib.rs`でpub mod math、pub mod traits、pub mod typesを宣言
  - `math/mod.rs`でpub mod smoothingを宣言
  - `types/mod.rs`でpub mod dual、pub mod timeを宣言
  - `traits/mod.rs`を作成 (個別のサブモジュールなし)
  - 各mod.rsに`//!`形式のモジュールドキュメントコメントを追加
  - _Requirements: 5, 7_

- [x] 1.3 (P) lib.rsにクレートレベルのドキュメントを記述する
  - Layer 1の役割 (Foundation層、数学的基盤の提供) を説明
  - 依存関係ゼロの原則 (他のpricer_*クレートに依存しない) を記載
  - 安定版Rustツールチェインのみでビルド可能であることを明記
  - 使用例 (f64とDualNumberでsmooth_maxを呼び出す例) を含める
  - _Requirements: 5, 7_

- [x] 2. スムージング関数を実装する
- [x] 2.1 smooth_max関数を実装する
  - LogSumExp公式を使用: `smooth_max(a, b, ε) = ε * log(exp(a/ε) + exp(b/ε))`
  - ジェネリック型パラメータ`T: num_traits::Float`でf32/f64をサポート
  - `#[inline]`属性を適用してLLVM最適化を有効化
  - epsilon > 0をassert!で検証し、違反時にパニック
  - Rustdocコメントに数学的定義、収束性、推奨範囲 (1e-8 ~ 1e-3)、パニック条件を記載
  - _Requirements: 1, 7, 10_

- [x] 2.2 smooth_min関数を実装する
  - smooth_maxの双対として実装: `smooth_min(a, b, ε) = -smooth_max(-a, -b, ε)`
  - ジェネリック型パラメータ`T: num_traits::Float`を使用
  - `#[inline]`属性を適用
  - epsilon > 0の検証を実施
  - Rustdocコメントに数学的定義と収束性を記載
  - _Requirements: 1, 7, 10_

- [x] 2.3 smooth_indicator関数を実装する
  - Sigmoid公式を使用: `smooth_indicator(x, ε) = 1 / (1 + exp(-x/ε))`
  - ジェネリック型パラメータ`T: num_traits::Float`を使用
  - `#[inline]`属性を適用
  - epsilon > 0の検証を実施
  - Rustdocコメントに数学的定義、収束性 (x < 0 → 0, x = 0 → 0.5, x > 0 → 1) を記載
  - _Requirements: 1, 7, 10_

- [x] 2.4 smooth_abs関数を実装する
  - Softplus公式を使用: `smooth_abs(x, ε) = ε * log(exp(x/ε) + exp(-x/ε))`
  - ジェネリック型パラメータ`T: num_traits::Float`を使用
  - `#[inline]`属性を適用
  - epsilon > 0の検証を実施
  - Rustdocコメントに数学的定義と収束性を記載
  - _Requirements: 1, 7, 10_

- [x] 2.5 スムージング関数のユニットテストを作成する
  - `#[cfg(test)]`モジュールをsmoothing.rsに配置
  - smooth_max収束性: epsilon = [1e-2, 1e-4, 1e-6]で真のmax関数との誤差を検証
  - smooth_min双対性: `smooth_min(a, b, ε) == -smooth_max(-a, -b, ε)`を検証
  - smooth_indicator境界値: `smooth_indicator(0, ε) ≈ 0.5`を検証 (approx使用)
  - smooth_abs偶関数性: `smooth_abs(-x, ε) == smooth_abs(x, ε)`を検証 (approx使用)
  - epsilon <= 0でパニックすることを検証
  - _Requirements: 6, 7_

- [x] 3. Dual数型統合を実装する
- [x] 3.1 (P) types::dualモジュールを実装する
  - `pub type DualNumber = num_dual::Dual64;`型エイリアスを定義
  - Rustdocコメントに使用例 (DualNumberでsmooth_maxを呼び出す例) を記載
  - モジュールドキュメントにnum-dualクレート統合の目的を説明
  - _Requirements: 2, 7_

- [x] 3.2 (P) Dual数型とスムージング関数の統合テストを作成する
  - 全てのスムージング関数 (smooth_max, smooth_min, smooth_indicator, smooth_abs) をDualNumberで呼び出し
  - 勾配情報が正しく伝播することを検証 (result.epsが有限値)
  - smooth_maxの解析的微分 (`∂smooth_max/∂a`) とDual数の勾配を比較 (approx使用)
  - num_traitsトレイト経由でDual数の基本演算 (加算、減算、乗算、除算、exp、log) が動作することを検証
  - _Requirements: 2, 6_

- [x] 4. 基本トレイトを定義する
- [x] 4.1 (P) Priceableトレイトを定義する
  - ジェネリック型パラメータ`T: num_traits::Float`を使用
  - `fn price(&self) -> T`メソッドを宣言
  - Rustdocコメントに使用例 (Layer 2のenum Instrumentでの実装例) を記載
  - 静的ディスパッチの要件を明記し、`Box<dyn Priceable>`を禁止
  - Enzyme最適化との互換性を説明
  - _Requirements: 3, 7_

- [x] 4.2 (P) Differentiableトレイトを定義する
  - ジェネリック型パラメータ`T: num_traits::Float`を使用
  - `fn gradient(&self) -> T`メソッドを宣言
  - Rustdocコメントに使用例 (Layer 3のADエンジンでの使用) を記載
  - 静的ディスパッチの要件とEnzyme/Dual数での実装パターンを説明
  - _Requirements: 3, 7_

- [x] 4.3 (P) トレイトのドキュメントテストを作成する
  - Priceableトレイトをf32、f64、DualNumberで実装するサンプルコードをdoctestで検証
  - 静的ディスパッチ (enum) のパターンが正しく動作することを確認
  - トレイトメソッドが副作用を持たないことを文書化
  - _Requirements: 3, 7_

- [x] 5. 時間型とDay Count Conventionを実装する
- [x] 5.1 (P) DayCountConvention列挙型を定義する
  - `#[non_exhaustive]`、`#[derive(Debug, Clone, Copy, PartialEq, Eq)]`を適用
  - バリアント: ActualActual365、ActualActual360、Thirty360を定義
  - Rustdocコメントに各規約の用途 (デリバティブ、マネーマーケット、米国社債) を記載
  - _Requirements: 4, 7_

- [x] 5.2 (P) DayCountConvention::year_fraction関数を実装する
  - `fn year_fraction(&self, start: NaiveDate, end: NaiveDate) -> f64`メソッドを実装
  - ActualActual365: `(end - start).num_days() / 365.0`
  - ActualActual360: `(end - start).num_days() / 360.0`
  - Thirty360: 30/360 US Bond Basis規約 (design.mdの実装詳細を参照)
  - start > endの場合にパニックまたは負の値を返す (ドキュメントで明記)
  - _Requirements: 4, 7_

- [x] 5.3 (P) time_to_maturity関数を実装する
  - `pub fn time_to_maturity(start: NaiveDate, end: NaiveDate) -> f64`関数を実装
  - デフォルト規約 (Act/365) を使用: `DayCountConvention::ActualActual365.year_fraction(start, end)`
  - Rustdocコメントにデフォルト規約と使用例を記載
  - _Requirements: 4, 7_

- [x] 5.4 (P) Day Count Conventionのユニットテストを作成する
  - 既知の日付ペア (2024-01-01 → 2024-07-01) で各規約の結果を手計算と比較
  - Act/365: 182 / 365.0 ≈ 0.4986
  - Act/360: 182 / 360.0 ≈ 0.5056
  - Thirty360: 米国規約での計算結果を検証
  - time_to_maturityがAct/365と一致することを検証
  - start > endで適切なエラー処理が行われることを検証
  - _Requirements: 4, 6_

- [x] 6. プロパティテストを実装する
- [x] 6.1 (P) スムージング関数のプロパティテストを作成する
  - proptestを使用してランダムな入力 (a, b, epsilon) を生成
  - epsilonの範囲を1e-8 ~ 1e-3に制限
  - smooth_max不等式: `smooth_max(a, b, ε) >= max(a, b) - tolerance`を検証
  - smooth_min不等式: `smooth_min(a, b, ε) <= min(a, b) + tolerance`を検証
  - smooth_max交換法則: `smooth_max(a, b, ε) == smooth_max(b, a, ε)`を検証
  - smooth_indicator単調性: `x1 < x2 → smooth_indicator(x1, ε) <= smooth_indicator(x2, ε)`を検証
  - cases = 1000で十分な試行回数を確保
  - _Requirements: 6_

- [x] 6.2 (P) Day Count Conventionのプロパティテストを作成する
  - proptestでランダムな日付ペア (start, end) を生成 (start <= end)
  - 全ての規約でyear_fractionが非負であることを検証
  - Act/365とAct/360の結果がおおよそ360/365の比率になることを検証
  - time_to_maturityがAct/365と常に一致することを検証
  - _Requirements: 6_

- [x] 7. コード品質チェックとビルド検証を実施する
- [x] 7.1 cargo fmtとcargo clippyを実行する
  - `cargo fmt --all -- --check`で全てのコードがフォーマット済みであることを確認
  - `cargo clippy --all-targets -- -D warnings`で全てのlint警告を解消
  - 警告が出た場合は修正し、再度検証
  - _Requirements: 7_

- [x] 7.2 安定版Rustツールチェインでビルドを検証する
  - `cargo build -p pricer_core`でビルドが成功することを確認 (ナイトリー不要)
  - `cargo test -p pricer_core`で全てのテストが成功することを確認
  - `cargo doc --no-deps -p pricer_core`でドキュメント生成が成功し、リンク切れがないことを確認
  - `cargo test --doc -p pricer_core`でドキュメント内のコード例が動作することを確認
  - _Requirements: 7, 8_

- [x] 7.3 依存関係ツリーを検証する
  - `cargo tree -p pricer_core`で依存関係ツリーを確認
  - 他のpricer_*クレートへの依存がないことを確認
  - 外部依存がnum-traits、num-dual、chrono、テストクレートのみであることを確認
  - 不要な推移的依存が含まれていないことを確認
  - _Requirements: 9_

- [x] 8. 統合テストとドキュメント最終化を実施する
- [x] 8.1 (P) モジュールエクスポートの統合テストを作成する
  - lib.rsからの絶対パスインポート (`use pricer_core::math::smoothing::smooth_max`) が動作することを検証
  - 各モジュール (math, traits, types) が正しく公開されていることを確認
  - chronoとの統合 (NaiveDateからtime_to_maturity) が動作することを検証
  - _Requirements: 5, 6_

- [x] 8.2 (P) 全体的なドキュメント品質を確認する
  - 全てのpublic関数とトレイトにRustdocコメント (///) が含まれていることを確認
  - 各モジュールに`//!`形式のモジュールドキュメントが含まれていることを確認
  - lib.rsのクレートレベルドキュメントが完全であることを確認
  - ドキュメント内の使用例が正しく動作することを確認 (cargo test --doc)
  - _Requirements: 7_

- [x] 8.3 最終的なビルドとテストの実行
  - `cargo build --workspace --exclude pricer_kernel`で安定版クレートのみがビルドされることを確認
  - `cargo test --workspace --exclude pricer_kernel`で全てのテストが成功することを確認
  - テスト失敗時にproptestが失敗ケースの入力値を報告することを確認
  - ビルドエラー時にコンパイルエラーメッセージが明確に表示されることを確認
  - _Requirements: 8_

## Requirements Coverage

全ての要件がタスクにマッピングされています:

- **Requirement 1** (スムージング関数の実装): Tasks 2.1-2.5
- **Requirement 2** (Dual数型とnum-dual統合): Tasks 1.1, 3.1-3.2
- **Requirement 3** (基本的なトレイト定義): Tasks 4.1-4.3
- **Requirement 4** (時間型と日付計算): Tasks 1.1, 5.1-5.4
- **Requirement 5** (モジュール構造とエクスポート): Tasks 1.2-1.3, 8.1
- **Requirement 6** (テスト戦略とプロパティテスト): Tasks 2.5, 3.2, 5.4, 6.1-6.2, 8.1
- **Requirement 7** (ドキュメントとコードスタイル): Tasks 1.2-1.3, 2.1-2.4, 3.1, 4.1-4.3, 5.1-5.3, 7.1-7.2, 8.2
- **Requirement 8** (ビルドとCI統合): Tasks 1.1, 7.2, 8.3
- **Requirement 9** (依存関係の最小化): Tasks 1.1, 7.3
- **Requirement 10** (パフォーマンスとメモリ効率): Tasks 2.1-2.4

## Parallel Execution Notes

以下のタスクは並列実行可能です (P) マーカー付き:
- **Task 1.1-1.3**: プロジェクト構造の初期化 (独立したファイル操作)
- **Task 3.1, 4.1-4.3, 5.1-5.4**: 独立したモジュールの実装 (ファイル競合なし)
- **Task 6.1-6.2, 8.1-8.2**: テストとドキュメントの作成 (独立した検証作業)

以下のタスクは順序依存のため順次実行が必要です:
- **Task 2.1-2.5**: スムージング関数の実装とテスト (2.5は2.1-2.4に依存)
- **Task 3.2**: Dual数型統合テスト (2.1-2.4と3.1に依存)
- **Task 7.1-7.3, 8.3**: 品質チェックとビルド検証 (全ての実装完了後)
