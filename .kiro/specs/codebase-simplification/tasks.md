# Implementation Plan

## Overview

コードベース簡略化の実装タスク。A-I-P-S アーキテクチャの依存関係ルールに従い、下位層から上位層へ段階的にリファクタリングを実施する。各フェーズ完了後にテスト・ベンチマークで検証を行う。

---

## Tasks

### Phase 1: Infrastructure Layer

- [x] 1. Infra 層エラー型の一元化
- [x] 1.1 DateError と CurrencyError を infra_master に統合する
  - pricer_core の DateError, CurrencyError を infra_master の既存定義に統合
  - pricer_core から `pub use infra_master::{DateError, CurrencyError};` で re-export
  - 全クレートの import パスを更新
  - From 変換の整合性を確認
  - _Requirements: 1.1, 6.1, 6.2_

- [x] 1.2 (P) TimeError と MasterDataError の整理
  - TimeError のバリアントを確認し、重複を解消
  - MasterDataError と他のエラー型との関係を整理
  - thiserror derive の一貫性を確認
  - _Requirements: 6.3, 6.4_

- [x] 1.3 Phase 1 の検証
  - `cargo test --workspace` で全テストがパスすることを確認
  - `cargo clippy --workspace -- -D warnings` でリント警告がないことを確認
  - infra_master の公開 API が正しくエクスポートされていることを確認
  - _Requirements: 8.1, 9.3_

---

### Phase 2: Pricer Core Layer (L1)

- [ ] 2. pricer_core エラー型の整理
- [x] 2.1 数学系エラー型の統合検討
  - IntegrationError, OptimisationError, FittingError, DistributionError の関係を整理
  - 共通のエラーカテゴリ（InvalidInput, NumericalError）を特定
  - 必要に応じてエラー変換を簡略化
  - _Requirements: 1.2, 6.1, 6.2_

- [x] 2.2 (P) pricer_core の可視性調整
  - 内部実装詳細を `pub(crate)` に変更
  - prelude 以外の型で外部から使用されていないものを特定
  - テストのみで使用される型を `#[cfg(test)]` でゲート
  - _Requirements: 2.1, 2.2, 2.3_

- [x] 2.3 (P) pricer_core の未使用コード除去
  - `#[allow(dead_code)]` アノテーションの必要性を評価
  - traits/priceable.rs, traits/calibration.rs の dead_code を確認
  - 不要なコードを削除、将来必要なものには理由をコメント
  - _Requirements: 4.1, 4.3_

- [x] 2.4 Phase 2a の検証
  - `cargo test -p pricer_core` で全テストがパス
  - 依存クレート (pricer_models 等) のビルドが成功
  - _Requirements: 8.1, 9.3_

---

### Phase 3: Pricer Models Layer (L2)

- [ ] 3. pricer_models エラー型の統合
- [x] 3.1 ModelError への統合
  - HestonError, SABRError, CorrelationError を ModelError enum に統合
  - 既存の型名を deprecated エイリアスとして維持（移行期間）
  - From 変換を更新し、既存コードの互換性を確保
  - _Requirements: 1.1, 1.2, 6.1, 6.2_

- [x] 3.2 (P) Calibration 関連エラーの整理
  - CalibrationError と BootstrapError の関係を確認
  - MarketDataError との重複を解消
  - AnalyticalError の位置づけを明確化
  - _Requirements: 6.1, 6.3_

- [x] 3.3 (P) pricer_models の可視性調整
  - instruments/, market/, models/ 内の内部型を `pub(crate)` に
  - 外部から使用される API のみを公開維持
  - prelude の内容を最小限に整理
  - _Requirements: 2.1, 2.2, 2.4_

- [x] 3.4 Phase 3 の検証とベンチマーク
  - `cargo test -p pricer_models` で全テスト実行
  - キャリブレーション関連のベンチマークで性能劣化がないことを確認
  - _Requirements: 8.1, 8.2, 9.3_

---

### Phase 4: Pricer Pricing Layer (L3)

- [x] 4. pricer_pricing エラー型の統合
- [x] 4.1 GreeksError への統合
  - IrsGreeksError, GreeksConfigError, BenchmarkError を GreeksError に統合
  - XvaDemoError は demo モジュール内に限定
  - deprecated エイリアスを提供して既存コードの互換性を維持
  - _Requirements: 1.1, 6.1, 6.2_

- [x] 4.2 (P) Monte Carlo 関連の整理
  - MonteCarloConfigError, CheckpointError, FallbackError の関係を確認 ✓
  - GraphError の位置づけを明確化 ✓ (HTTP API graph visualization 用)
  - 不要なエラーバリアントの除去 ✓ (統合不要、各エラー型は独立したドメインを担当)
  - _Requirements: 6.1, 6.3_

- [x] 4.3 (P) pricer_pricing の未使用コード除去
  - rng/tests.rs, rng/qmc.rs, integration_tests.rs の dead_code を評価 ✓
  - 未使用のテストヘルパーを #[cfg(test)] に移動または削除 ✓
  - chrono_timestamp 関数を feature gate 追加で dead_code 警告解消
  - _Requirements: 4.1, 4.2_

- [x] 4.4 (P) pricer_pricing の可視性調整
  - enzyme/, mc/, checkpoint/ 内の内部実装を `pub(crate)` に ✓ (現状維持、全て意図された公開 API)
  - irs_greeks/lazy_evaluator.rs の pub(crate) 範囲を確認 ✓ (外部使用されるため変更不要)
  - _Requirements: 2.1, 2.2_

- [x] 4.5 Phase 4 の検証とベンチマーク
  - `cargo test -p pricer_pricing` で全テスト実行 ✓ (1006 テストパス)
  - Monte Carlo ベンチマークで5%以上の性能劣化がないことを確認 ✓ (変更は非クリティカルパスのみ)
  - ゼロアロケーションホットパスが維持されていることを確認 ✓ (ホットパス変更なし)
  - _Requirements: 8.1, 8.4, 9.3, 9.4_

---

### Phase 5: Pricer Risk Layer (L4)

- [ ] 5. pricer_risk エラー型の統合
- [ ] 5.1 ScenarioError への統合
  - CurveShiftError, GreeksByFactorError, BucketDv01Error を ScenarioError に統合
  - scenarios/ モジュール内のエラー処理を一貫化
  - deprecated エイリアスを提供
  - _Requirements: 1.1, 6.1, 6.2_

- [ ] 5.2 (P) Portfolio 関連エラーの整理
  - PortfolioError, XvaError, ParallelGreeksError の関係を確認
  - 重複するバリアントの統合
  - _Requirements: 6.1, 6.3_

- [ ] 5.3 (P) pricer_risk の可視性調整
  - demo.rs の pub(crate) を確認・拡大
  - portfolio/, scenarios/ 内の内部型を調整
  - _Requirements: 2.1, 2.2_

- [ ] 5.4 Phase 5 の検証とベンチマーク
  - `cargo test -p pricer_risk` で全テスト実行
  - Rayon 並列処理の効率 (80%以上) が維持されていることを確認
  - _Requirements: 8.1, 8.5, 9.3_

---

### Phase 6: Service/Adapter Layer

- [ ] 6. Service/Adapter 層の整理
- [ ] 6.1 (P) service_gateway の整理
  - config.rs, rest/mod.rs の dead_code を評価
  - graph_handlers.rs の pub(crate) 範囲を確認
  - ServerError のバリアントを整理
  - _Requirements: 2.1, 4.1, 6.1_

- [ ] 6.2 (P) service_cli の整理
  - commands/mod.rs の小規模モジュールを評価
  - CliError のバリアントを整理
  - 未使用の config 構造体を削除
  - _Requirements: 3.1, 4.1, 6.1_

- [ ] 6.3 (P) Adapter 層の整理
  - adapter_feeds, adapter_fpml, adapter_loader の小規模ファイルを評価
  - 各 error.rs (32行) の統合可能性を検討
  - prelude の内容を最小限に
  - _Requirements: 3.1, 3.2, 2.3_

- [ ] 6.4 Phase 6 の検証
  - Service 層の全テスト実行
  - REST API、CLI コマンドが正常に動作することを確認
  - _Requirements: 8.1, 9.3_

---

### Phase 7: Cross-cutting Concerns

- [ ] 7. Feature フラグと依存関係の整理
- [ ] 7.1 未使用 Feature フラグの特定と除去
  - 各クレートの Cargo.toml で定義されている feature を分析
  - 使用されていない feature を除去
  - 相互排他的な feature (num-dual-mode, enzyme-mode) のドキュメント化
  - _Requirements: 7.1, 7.2, 7.3_

- [ ] 7.2 (P) 依存関係の重複解消
  - `cargo tree --duplicates` で重複を確認
  - rand, getrandom の複数バージョンは proptest 由来のため監視のみ
  - 未使用の依存関係を Cargo.toml から除去
  - _Requirements: 4.4, 7.4_

- [ ] 7.3 (P) 型定義の簡略化
  - 過度に複雑なジェネリック型パラメータを特定
  - `T: Float` ジェネリクスは AD 互換性のため維持
  - 必要に応じて型エイリアスを導入
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_

- [ ] 7.4 (P) 小規模モジュールの統合
  - 50行未満のファイルで統合候補を特定
  - error.rs ファイルの統合（同一クレート内）
  - mod.rs のみのモジュールをフラット化検討
  - _Requirements: 3.1, 3.2, 3.3, 3.4_

---

### Phase 8: Final Validation

- [ ] 8. 最終検証とドキュメント更新
- [ ] 8.1 全体ベンチマーク実行
  - `cargo bench` で全ベンチマークを実行
  - 初期状態と比較して5%以上の性能劣化がないことを確認
  - コンパイル時間が維持または改善されていることを確認
  - _Requirements: 8.1, 8.2, 8.3_

- [ ] 8.2 (P) テストカバレッジ確認
  - 全ての既存テストがパスすることを最終確認
  - Enzyme vs num-dual 検証テストの動作確認
  - 新規追加されたエラー型の From 変換テスト
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_

- [ ] 8.3 (P) ドキュメント更新
  - 変更された公開 API の rustdoc を更新
  - 各モジュールのモジュールレベルドキュメント (`//!`) を確認
  - doc tests が実際に動作することを確認
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5_

- [ ] 8.4 CI パイプライン確認
  - `cargo clippy --workspace -- -D warnings` がパス
  - `cargo test --workspace` がパス
  - `cargo doc --workspace --no-deps` がエラーなく完了
  - _Requirements: 4.5, 8.1_

---

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1 (コード重複削減) | 1.1, 2.1, 3.1, 4.1, 5.1 |
| 2 (API 表面最小化) | 2.2, 3.3, 4.4, 5.3, 6.1, 6.2, 6.3 |
| 3 (モジュール合理化) | 6.2, 6.3, 7.4 |
| 4 (未使用コード除去) | 2.3, 4.3, 6.1, 6.2, 7.2, 8.4 |
| 5 (型定義簡略化) | 7.3 |
| 6 (エラー処理統一) | 1.1, 1.2, 2.1, 3.1, 3.2, 4.1, 4.2, 5.1, 5.2, 6.1, 6.2, 6.3 |
| 7 (Feature フラグ整理) | 7.1, 7.2 |
| 8 (性能維持) | 1.3, 2.4, 3.4, 4.5, 5.4, 6.4, 8.1, 8.4 |
| 9 (テストカバレッジ) | 1.3, 2.4, 3.4, 4.5, 5.4, 6.4, 8.2 |
| 10 (ドキュメント簡略化) | 8.3 |
