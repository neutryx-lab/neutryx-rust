# Requirements Document

## Introduction

本仕様は、Neutryx デリバティブ価格計算ライブラリのコードベース全体を、機能や性能を維持しながら徹底的に簡略化することを目的とする。A-I-P-S アーキテクチャの原則を遵守しつつ、コードの可読性、保守性、コンパイル時間を改善する。

## Requirements

### Requirement 1: コード重複の削減

**Objective:** As a 開発者, I want コードベース全体で重複するパターンを統合する, so that 保守性が向上し変更箇所が減少する

#### Acceptance Criteria
1. When 同一の数学関数が複数のクレートに存在する場合, the システム shall 共通の実装を `pricer_core::math` に集約する
2. When 類似のエラーハンドリングパターンが検出された場合, the システム shall 共通のエラー型またはマクロを提供する
3. When 重複するテストユーティリティが存在する場合, the システム shall 共有テストヘルパーモジュールを提供する
4. The システム shall 重複コード削減後も全ての既存テストがパスすること
5. While 簡略化を実施する間, the システム shall A-I-P-S 依存関係ルールを維持すること

### Requirement 2: API 表面の最小化

**Objective:** As a ライブラリ利用者, I want シンプルで一貫性のある公開 API を使用する, so that 学習コストが低減しミス使用が防止される

#### Acceptance Criteria
1. When 公開型が内部実装詳細を露出している場合, the システム shall 適切な可視性修飾子 (pub(crate), pub(super)) を適用する
2. When 同一機能に対して複数の公開関数が存在する場合, the システム shall 単一の明確な API を提供する
3. When prelude モジュールが肥大化している場合, the システム shall 最も頻繁に使用される型のみをエクスポートする
4. The システム shall 公開 API の変更に対してドキュメンテーションを更新すること
5. Where 破壊的変更が必要な場合, the システム shall 非推奨アノテーションと移行ガイドを提供する

### Requirement 3: モジュール構造の合理化

**Objective:** As a 開発者, I want 論理的に整理されたモジュール構造を持つ, so that コードナビゲーションと理解が容易になる

#### Acceptance Criteria
1. When 小さすぎるモジュール (50行未満) が単独で存在する場合, the システム shall 関連モジュールへの統合を検討する
2. When モジュール間の循環依存が検出された場合, the システム shall 依存関係を整理して解消する
3. When 深すぎるネスト (3階層以上) が存在する場合, the システム shall フラット化を検討する
4. The システム shall 各モジュールに明確な単一責任を持たせること
5. While モジュール再構成を行う間, the システム shall 既存のインポートパスに対する互換性レイヤーを提供すること

### Requirement 4: 未使用コードの除去

**Objective:** As a 開発者, I want 不要なコードを除去する, so that コードベースが軽量化されコンパイル時間が短縮される

#### Acceptance Criteria
1. When `#[allow(dead_code)]` アノテーションが付与されたコードが1ヶ月以上未使用の場合, the システム shall 除去対象として特定する
2. When feature フラグでゲートされた機能が使用されていない場合, the システム shall 除去または統合を検討する
3. When 非推奨 (deprecated) としてマークされた API が存在する場合, the システム shall 除去スケジュールを明確にする
4. The システム shall 未使用の依存関係をCargo.toml から除去すること
5. The システム shall 未使用コード除去後もCI パイプラインが全て成功すること

### Requirement 5: 型定義の簡略化

**Objective:** As a 開発者, I want シンプルで理解しやすい型定義を使用する, so that コードの可読性が向上する

#### Acceptance Criteria
1. When 過度に複雑なジェネリック型パラメータが存在する場合, the システム shall 型エイリアスまたは具象型への置換を検討する
2. When 類似の enum バリアントが複数存在する場合, the システム shall 共通の抽象化を提供する
3. When newtype パターンが過剰に使用されている場合, the システム shall 必要性を再評価する
4. The システム shall 型の簡略化後も型安全性を維持すること
5. Where AD (自動微分) 互換性が必要な場合, the システム shall `T: Float` ジェネリクスを維持すること

### Requirement 6: エラー処理の統一

**Objective:** As a 開発者, I want 一貫したエラー処理パターンを使用する, so that デバッグが容易になる

#### Acceptance Criteria
1. When 各クレートで異なるエラー型パターンが使用されている場合, the システム shall 共通のエラー設計ガイドラインに従う
2. When エラー変換 (From 実装) が複雑になっている場合, the システム shall エラーチェーンを簡略化する
3. The システム shall 全てのエラー型が `thiserror` を使用すること
4. The システム shall エラーメッセージが問題の診断に十分な情報を含むこと
5. Where serde feature が有効な場合, the システム shall エラー型のシリアライゼーションをサポートする

### Requirement 7: Feature フラグの整理

**Objective:** As a ビルド管理者, I want 明確で管理しやすい feature フラグ構造を持つ, so that ビルド構成が簡潔になる

#### Acceptance Criteria
1. When 使用されていない feature フラグが存在する場合, the システム shall 除去する
2. When feature フラグの組み合わせが複雑すぎる場合, the システム shall 整理統合する
3. The システム shall 相互排他的な feature フラグを明確に文書化すること
4. The システム shall デフォルト feature セットで基本機能が動作すること
5. While feature 整理を行う間, the システム shall 既存のビルド構成との互換性を維持すること

### Requirement 8: 性能の維持

**Objective:** As a 利用者, I want 簡略化後も同等以上の性能を得る, so that 本番環境での使用に影響がない

#### Acceptance Criteria
1. When 簡略化変更を適用した場合, the システム shall 既存ベンチマークで5%以上の性能劣化がないこと
2. When コンパイル時間に影響する変更を行った場合, the システム shall インクリメンタルビルド時間を維持または改善すること
3. The システム shall リリースビルドのバイナリサイズを増加させないこと
4. Where Monte Carlo シミュレーションを使用する場合, the システム shall ゼロアロケーションホットパスを維持すること
5. The システム shall Rayon 並列処理の効率 (8コア以上で80%以上) を維持すること

### Requirement 9: テストカバレッジの確保

**Objective:** As a 品質保証担当者, I want 簡略化後もテストカバレッジが維持される, so that リグレッションが防止される

#### Acceptance Criteria
1. When コードを削除または統合した場合, the システム shall 対応するテストを更新または移動すること
2. When 新しい共通モジュールを作成した場合, the システム shall 単体テストを追加すること
3. The システム shall 全ての既存統合テストがパスすること
4. The システム shall Enzyme vs num-dual 検証テストを維持すること
5. While テストを更新する間, the システム shall テストの意図と目的を明確に文書化すること

### Requirement 10: ドキュメンテーションの簡略化

**Objective:** As a 新規開発者, I want 最新で簡潔なドキュメントを参照する, so that オンボーディングが迅速になる

#### Acceptance Criteria
1. When API が変更された場合, the システム shall 対応する rustdoc を更新すること
2. When 古いドキュメントが存在する場合, the システム shall 除去または更新すること
3. The システム shall 各公開モジュールにモジュールレベルドキュメント (`//!`) を持つこと
4. The システム shall コード例 (doc tests) が実際に動作すること
5. Where 複雑なアルゴリズムを実装している場合, the システム shall 参考文献または数式を含むこと
