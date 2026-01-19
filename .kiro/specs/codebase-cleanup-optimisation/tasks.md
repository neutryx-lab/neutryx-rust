# Implementation Plan

## Phase 1: Comment Cleanup

- [ ] 1. TODO/FIXMEコメントの削除
- [x] 1.1 (P) Service層のTODOコメントを削除する
  - service_gatewayのhandlers.rs、main.rsからTODOコメントを削除
  - service_cliのreport.rs、price.rs、calibrate.rsからTODOコメントを削除
  - 削除後にcargo fmt/clippy/testで品質検証
  - _Requirements: 1.1, 1.2_

- [x] 1.2 (P) Adapter層のTODOコメントを削除する
  - adapter_fpmlのparser.rsからFpML parsing関連のTODOコメントを削除
  - 削除後にcargo fmt/clippy/testで品質検証
  - _Requirements: 1.1, 1.2_

- [x] 1.3 (P) Pricer層のTODOコメントを削除する
  - pricer_pricingのirs_greeks/calculator.rs、xva_demo.rsからTODOコメントを削除
  - 削除後にcargo fmt/clippy/testで品質検証
  - _Requirements: 1.1, 1.2_

- [ ] 2. コメントアウトされたコードの削除
- [x] 2.1 (P) verify_enzyme.rsのコメントアウトコードを削除する
  - 使用されていないコメントアウトされた変数宣言を削除
  - 数学的導出を説明するコメント（`// f(x) = ...`等）は保持
  - 削除後にcargo fmt/clippy/testで品質検証
  - _Requirements: 1.3, 1.4_

- [x] 2.2 (P) pricer_checkpoint.rsのコメントアウトコードを削除する
  - コメントアウトされたループ実装を削除
  - アルゴリズム説明コメントは保持
  - 削除後にcargo fmt/clippy/testで品質検証
  - _Requirements: 1.3, 1.4_

- [ ] 3. Phase 1完了検証
  - cargo fmt --all -- --checkを実行
  - cargo clippy --all-targets -- -D warningsを実行
  - cargo test --workspaceで回帰テスト実行
  - _Requirements: 5.1, 5.2, 5.3_

## Phase 2: Dead Code Removal

- [ ] 4. #[allow(dead_code)]アトリビュートの削除と検証
- [ ] 4.1 (P) Service層のdead code許容を削除する
  - service_gateway/src/rest/handlers.rs、config.rsから#[allow(dead_code)]を削除
  - service_cli/src/config.rsから#[allow(dead_code)]を削除
  - 未使用と判明したコードは削除、必要なコードは警告を解消
  - _Requirements: 6.2, 6.3_

- [ ] 4.2 (P) Pricer層のdead code許容を削除する
  - pricer_pricingのrng/qmc.rs、enzyme/checkpoint_ad.rsから#[allow(dead_code)]を削除
  - pricer_optimiserのbootstrapping/curve_builder.rsから#[allow(dead_code)]を削除
  - pricer_modelsのcalibration/model_calibrator.rsから#[allow(dead_code)]を削除
  - _Requirements: 6.2, 6.3_

- [ ] 5. clippy未使用警告による追加dead code検出
  - cargo clippy --all-targets -- -W dead_code -W unused_importsを実行
  - 検出された未使用コードを評価し、不要なものを削除
  - API互換性のために必要な公開インターフェースは保持
  - _Requirements: 6.3, 6.7_

- [ ] 6. 1-2行ラッパー関数のインライン化検討
  - 単純な委譲のみを行うラッパー関数を特定
  - 可読性が向上する場合のみインライン化を実施
  - _Requirements: 6.2, 6.6_

- [ ] 7. Phase 2完了検証
  - cargo fmt/clippy/testを実行
  - dead_code警告がゼロであることを確認
  - _Requirements: 5.1, 5.2, 5.3, 6.3_

## Phase 3: Error Handling Refactor

- [ ] 8. pricer_coreのunwrap/expect排除
- [ ] 8.1 pricer_core/types配下のunwrap排除
  - types/time.rs、currency_pair.rsのunwrap/expectをResult/Optionベースに変換
  - 既存エラー型を活用し、必要に応じてバリアントを追加
  - _Requirements: 5.6, 3.3_

- [ ] 8.2 pricer_core/math配下のunwrap排除
  - math/interpolators/*.rsのunwrap/expectをResult/Optionベースに変換
  - math/solvers/*.rsのunwrap/expectを変換
  - 境界チェックにはget()を使用
  - _Requirements: 5.6, 3.3_

- [ ] 8.3 pricer_core/market_data配下のunwrap排除
  - curves/*.rs、surfaces/*.rsのunwrap/expectを変換
  - 既存のMarketDataErrorを活用
  - _Requirements: 5.6, 3.3_

- [ ] 8.4 pricer_core変更後の品質検証
  - cargo test -p pricer_coreで全テスト通過を確認
  - 依存クレートのコンパイル確認
  - _Requirements: 5.1, 5.5_

- [ ] 9. pricer_modelsのunwrap/expect排除
- [ ] 9.1 pricer_models/schedules配下のunwrap排除
  - schedules/schedule.rs、period.rs、frequency.rsのunwrap/expectを変換
  - _Requirements: 5.6, 3.3_

- [ ] 9.2 pricer_models/analytical配下のunwrap排除
  - analytical/black_scholes.rs、bachelier.rs、garman_kohlhagen.rsのunwrap/expectを変換
  - 数値計算エラー（NaN、Inf）のハンドリングを追加
  - _Requirements: 5.6, 3.3_

- [ ] 9.3 pricer_models/models配下のunwrap排除
  - models/gbm.rs、stochastic.rsのunwrap/expectを変換
  - パラメータ検証エラーを適切に伝播
  - _Requirements: 5.6, 3.3_

- [ ] 9.4 pricer_models変更後の品質検証
  - cargo test -p pricer_modelsで全テスト通過を確認
  - 依存クレートのコンパイル確認
  - _Requirements: 5.1, 5.5_

- [ ] 10. pricer_optimiserのunwrap/expect排除
- [ ] 10.1 pricer_optimiser/bootstrapping配下のunwrap排除
  - bootstrapping/multi_curve.rs、sensitivity.rs、instrument.rsのunwrap/expectを変換
  - _Requirements: 5.6, 3.3_

- [ ] 10.2 pricer_optimiser/solversとprovider配下のunwrap排除
  - solvers/levenberg_marquardt.rs、bfgs.rsのunwrap/expectを変換
  - provider.rsのunwrap/expectを変換
  - _Requirements: 5.6, 3.3_

- [ ] 10.3 pricer_optimiser変更後の品質検証
  - cargo test -p pricer_optimiserで全テスト通過を確認
  - 統合テストの実行
  - _Requirements: 5.1, 5.5_

- [ ] 11. Phase 3完了検証
  - ライブラリコード内にunwrap/expect/panicが残っていないことを確認
  - 全cratesに対してcargo fmt/clippy/testを実行
  - _Requirements: 5.1, 5.2, 5.3, 5.6_

## Phase 4: Structure Optimisation

- [ ] 12. 大規模ファイルの構造評価
- [ ] 12.1 (P) sabr.rsとheston.rsの構造を評価する
  - sabr.rs（2,919行）とheston.rs（2,673行）の責務を分析
  - 分割により200行以上の独立モジュールが生成される場合のみ分割を検討
  - ミニマリズム原則と単一責任原則のバランスを評価
  - _Requirements: 2.1, 2.3, 6.5_

- [ ] 12.2 (P) pricer_core/types/time.rsの構造を評価する
  - time.rs（1,415行）のDate、DayCount、YearFractionの責務を分析
  - 関連性が高い場合は現状維持
  - _Requirements: 2.1, 2.3, 6.5_

- [ ] 12.3 (P) pricer_pricing/mc/pricer.rsの構造を評価する
  - pricer.rs（1,565行）の責務を分析
  - Monte Carloの核心機能が単一責任として妥当かを評価
  - _Requirements: 2.1, 2.3, 6.5_

- [ ] 13. 必要に応じたファイル分割の実施
  - Task 12で分割が妥当と判断されたファイルのみを対象に実施
  - mod.rsにpub use re-exportを追加し、外部APIを維持
  - 関連テストも適切に移動または更新
  - _Requirements: 2.3, 2.4, 2.5, 5.5_

- [ ] 14. 関数構造の最適化
- [ ] 14.1 深いネストを持つ関数のリファクタリング
  - 4段階以上のネストを持つ関数を特定
  - early returnパターンの適用でネストを削減
  - _Requirements: 3.2, 3.3_

- [ ] 14.2 pub可視性の適正化
  - モジュール外から呼び出されていない関数をpub(crate)またはprivateに変更
  - _Requirements: 3.6_

- [ ] 15. Phase 4完了検証
  - cargo fmt/clippy/testを実行
  - 外部APIが維持されていることを確認
  - _Requirements: 5.1, 5.2, 5.3, 5.5_

## Final Validation

- [ ] 16. 最終品質検証
  - 全要件（6要件、29受入基準）の充足を確認
  - cargo fmt --all -- --check合格
  - cargo clippy --all-targets -- -D warnings合格
  - cargo test --workspace全テスト通過
  - British English表記規則の準拠確認
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 6.1_
