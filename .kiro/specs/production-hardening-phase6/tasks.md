# Implementation Plan: Production Hardening Phase 6

## Tasks

- [ ] 1. ドキュメント品質設定の強化
- [ ] 1.1 (P) pricer_coreのドキュメントlint設定を強化する
  - 既存の`#![warn(missing_docs)]`を`#![deny(missing_docs)]`に変更
  - `#![deny(rustdoc::broken_intra_doc_links)]`を追加
  - 欠落しているdocコメントを補完し、コンパイルエラーを解消
  - 主要なパブリック関数に`# Examples`セクションを追加
  - _Requirements: 1.1, 1.5, 1.6_

- [ ] 1.2 (P) pricer_modelsのドキュメントlint設定を強化する
  - 既存の`#![warn(missing_docs)]`を`#![deny(missing_docs)]`に変更
  - `#![deny(rustdoc::broken_intra_doc_links)]`を追加
  - 欠落しているdocコメントを補完し、コンパイルエラーを解消
  - 主要なパブリック関数に`# Examples`セクションを追加
  - _Requirements: 1.2, 1.5, 1.6_

- [ ] 1.3 (P) pricer_kernelのドキュメントlint設定を強化する
  - 既存の`#![warn(missing_docs)]`を`#![deny(missing_docs)]`に変更
  - `#![deny(rustdoc::broken_intra_doc_links)]`を追加
  - 欠落しているdocコメントを補完し、コンパイルエラーを解消
  - モンテカルロ関連の関数に`# Examples`セクションを追加
  - _Requirements: 1.3, 1.5, 1.6_

- [ ] 1.4 (P) pricer_xvaのドキュメントlint設定を強化する
  - 既存の`#![warn(missing_docs)]`を`#![deny(missing_docs)]`に変更
  - `#![deny(rustdoc::broken_intra_doc_links)]`を追加
  - 欠落しているdocコメントを補完し、コンパイルエラーを解消
  - XVA計算関連の関数に`# Examples`セクションを追加
  - _Requirements: 1.4, 1.5, 1.6_

- [ ] 2. Criterionベンチマーク基盤の構築
- [ ] 2.1 pricer_coreのベンチマークを作成する
  - Cargo.tomlにcriterion v0.7.0を追加し、`[[bench]]`セクションを設定
  - 補間処理（linear, cubic_spline, bilinear）のベンチマークを実装
  - 異なるデータサイズ（100, 1000, 10000点）でのスケーリングを測定
  - `black_box`を使用してコンパイラ最適化を防止
  - _Requirements: 3.1, 3.6, 3.7_

- [ ] 2.2 pricer_modelsのベンチマークを作成する
  - Cargo.tomlにcriterion v0.7.0を追加し、`[[bench]]`セクションを設定
  - Black-Scholes価格計算（call/put）のベンチマークを実装
  - スポット価格、ストライク、ボラティリティの変動でのスケーリングを測定
  - 2.1の完了後に実施（Criterion設定パターンを参照）
  - _Requirements: 3.2, 3.6, 3.7_

- [ ] 2.3 pricer_kernelのベンチマークを作成する
  - Cargo.tomlにcriterion v0.7.0を追加し、`[[bench]]`セクションを設定
  - モンテカルロパス生成（1K, 10K, 100Kパス）のベンチマークを実装
  - Enzyme AD vs num-dual比較ベンチマークを実装
  - nightly toolchain環境での実行を考慮
  - 2.2の完了後に実施（依存パターンを参照）
  - _Requirements: 3.3, 3.4, 3.6, 3.7_

- [ ] 2.4 pricer_xvaのベンチマークを作成する
  - Cargo.tomlにcriterion v0.7.0を追加し、`[[bench]]`セクションを設定
  - ポートフォリオレベルXVA計算（CVA, exposure）のベンチマークを実装
  - 異なるポートフォリオサイズ（10, 100, 1000トレード）でのスケーリングを測定
  - 2.3の完了後に実施（全クレートのベンチマーク構成を統一）
  - _Requirements: 3.5, 3.6, 3.7_

- [ ] 3. CI/CDパイプラインの拡張
- [ ] 3.1 stable-layersジョブをクロスプラットフォーム対応にする
  - matrix戦略を追加してLinux, Windows, macOSで実行
  - OS固有の問題（パス区切り、改行コード）に対応
  - 各OSでのビルド・テスト・Clippyの成功を確認
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.6_

- [ ] 3.2 カバレッジ測定ジョブを追加する
  - cargo-tarpaulinをインストールするステップを追加
  - L1/L2/L4クレートのカバレッジを測定（L3は除外）
  - Codecovへのレポートアップロードを設定
  - stable-layersジョブ完了後に実行するよう依存関係を設定
  - _Requirements: 6.1, 6.2, 6.3, 6.4_

- [ ] 3.3 ベンチマーク実行ジョブを追加する
  - cargo benchを実行するステップを追加
  - L1/L2/L4クレートのベンチマークを実行（L3は条件付き）
  - ベンチマーク結果をアーティファクトとして保存
  - stable-layersジョブ完了後に実行するよう依存関係を設定
  - _Requirements: 3.6_

- [ ] 3.4 ドキュメント生成ジョブを強化する
  - `RUSTDOCFLAGS="-D warnings"`を設定してドキュメント警告をエラー化
  - mainブランチマージ時のドキュメントデプロイを維持
  - ドキュメント品質チェックをPRブロック条件に追加
  - _Requirements: 4.5, 6.5_

- [ ] 4. リリースワークフローの構築
- [ ] 4.1 git-cliff設定ファイルを作成する
  - cliff.tomlをリポジトリルートに作成
  - Conventional Commits形式でのグループ化を設定
  - 変更履歴のフォーマットテンプレートを定義
  - _Requirements: 7.3_

- [ ] 4.2 リリースワークフローを作成する
  - release.ymlを新規作成し、`v*`タグでトリガー
  - git-cliff-actionでCHANGELOG自動生成を実装
  - リリースビルド（L1/L2/L4）の生成ステップを追加
  - 将来のcrates.io公開用にコメントアウトしたpublishジョブを準備
  - 4.1の完了後に実施（cliff.toml設定を参照）
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_

- [ ] 5. 開発者体験の改善
- [ ] 5.1 (P) Issueテンプレートを作成する
  - バグレポート用テンプレートを作成（再現手順、環境情報）
  - 機能リクエスト用テンプレートを作成（ユースケース、期待動作）
  - config.ymlでテンプレート選択画面を設定
  - _Requirements: 8.5_

- [ ] 5.2 (P) PRテンプレートを作成する
  - 変更内容、テスト方法、関連Issueのセクションを含む
  - チェックリスト（テスト、フォーマット、Clippy）を追加
  - _Requirements: 8.5_

- [ ] 5.3 CONTRIBUTING.mdを拡張する
  - 開発環境セットアップ手順（stable/nightly分離）を追加
  - ローカルでのテスト実行方法を説明
  - PR作成からマージまでのフローを文書化
  - 30分以内のセットアップ完了を目標とした手順の簡潔化
  - 5.1, 5.2の完了後に実施（テンプレートへの参照を含む）
  - _Requirements: 8.1, 8.2, 8.3, 8.4_

- [ ] 6. 統合検証
- [ ] 6.1 ドキュメント生成の検証
  - `cargo doc --no-deps --workspace`を実行し、警告ゼロを確認
  - 生成されたHTMLドキュメントのExamplesセクションを確認
  - リンク切れがないことを検証
  - _Requirements: 1.5_

- [ ] 6.2 ベンチマーク実行の検証
  - `cargo bench --workspace --exclude pricer_kernel`を実行し、全ベンチマークが成功することを確認
  - HTMLレポートが`target/criterion/`に生成されることを確認
  - スケーリング特性が期待通りに測定されていることを確認
  - _Requirements: 3.6_

- [ ] 6.3 CI/CDパイプラインの検証
  - テストPRを作成し、全ジョブが正常に実行されることを確認
  - カバレッジレポートがCodecovにアップロードされることを確認
  - ベンチマーク結果がアーティファクトとして保存されることを確認
  - _Requirements: 4.1, 4.2, 6.1_

- [ ] 6.4 リリースワークフローの検証
  - テストタグ（v0.0.0-test）を作成し、release.ymlがトリガーされることを確認
  - CHANGELOGが正しく生成されることを確認
  - リリースビルドが成功することを確認
  - _Requirements: 7.2, 7.3_

---

## Requirements Coverage Summary

| Requirement | Tasks |
|-------------|-------|
| 1.1, 1.2, 1.3, 1.4, 1.5, 1.6 | 1.1, 1.2, 1.3, 1.4, 6.1 |
| 2.1, 2.2, 2.3, 2.4, 2.5 | 5.3（CONTRIBUTING.mdにセットアップ手順を含む） |
| 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7 | 2.1, 2.2, 2.3, 2.4, 3.3, 6.2 |
| 4.1, 4.2, 4.3, 4.4, 4.5, 4.6 | 3.1, 3.4, 6.3 |
| 5.1, 5.2, 5.3, 5.4, 5.5 | 既存CI維持（変更不要） |
| 6.1, 6.2, 6.3, 6.4, 6.5 | 3.2, 3.4, 6.3 |
| 7.1, 7.2, 7.3, 7.4, 7.5 | 4.1, 4.2, 6.4 |
| 8.1, 8.2, 8.3, 8.4, 8.5 | 5.1, 5.2, 5.3 |

---

## Notes

- **Requirement 5（Nightly CI）**: 既存のnightly-kernelジョブが要件を満たしているため、新規タスクは不要
- **Requirement 2（ユーザーガイド）**: README.mdは既に充実しており、CONTRIBUTING.md拡張で対応
- **並列実行可能タスク**: 1.1-1.4（クレート別ドキュメント強化）、5.1-5.2（テンプレート作成）
- **依存関係**: ベンチマークタスク（2.1→2.2→2.3→2.4）は順次実行を推奨（設定パターンの統一）
