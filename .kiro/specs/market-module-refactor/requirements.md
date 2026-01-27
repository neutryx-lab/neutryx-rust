# Requirements Document

## Introduction

`pricer_models::market`モジュールは、イールドカーブ、ボラティリティサーフェス、キャリブレーション、マーケットデータ管理など、量的金融の市場データ構造を提供する中核モジュールである。現在82ファイルが存在し、一部に冗長性や構造の複雑さが見られる。本リファクタリングは、不要ファイルの削除と構造の明確化を目的とする。

## Requirements

### Requirement 1: ファイル監査と不要ファイルの特定

**Objective:** As a 開発者, I want market/モジュール内の全ファイルを監査し不要なものを特定する, so that コードベースの保守性とビルド時間が改善される

#### Acceptance Criteria
1. The pricer_models shall 各.rsファイルの使用状況（他モジュールからの参照、pub export、テストでの使用）を分析し、未使用ファイルをリストアップする
2. When ファイルが他のどこからも参照されていない場合, the refactoring process shall そのファイルを削除候補としてマークする
3. The pricer_models shall `#[allow(dead_code)]`アノテーションが付いたコードを精査し、本当に必要かを判断する
4. If ファイルの機能が他ファイルと重複している場合, then the refactoring process shall 統合候補としてマークする
5. The pricer_models shall 削除・統合の決定を文書化し、変更の追跡を可能にする

### Requirement 2: calibration/サブモジュールの整理

**Objective:** As a 開発者, I want calibration/ディレクトリの構造を整理する, so that キャリブレーション関連コードの見通しが良くなる

#### Acceptance Criteria
1. The calibration module shall bootstrapping/サブモジュール内のファイル（現在16ファイル）を論理的なグループに整理する
2. When bootstrappingの機能が複数ファイルに分散している場合, the refactoring process shall 関連機能を適切に統合する
3. The calibration module shall error.rs、engine_error.rs等のエラー型を統合し、一貫したエラーハンドリング構造を提供する
4. The calibration module shall mod.rsのre-exportを整理し、公開APIを明確化する
5. If ファイルがテスト専用（proptest等）の場合, the refactoring process shall testsディレクトリへの移動を検討する

### Requirement 3: volcube/サブモジュールの整理

**Objective:** As a 開発者, I want volcube/ディレクトリ（現在22ファイル）を整理する, so that IRボラティリティキューブ機能の理解と保守が容易になる

#### Acceptance Criteria
1. The volcube module shall 論理的に関連するファイルをサブモジュールにグループ化する（例：calibration関連、cache関連、interpolation関連）
2. When ファイル名が機能を明確に示していない場合, the refactoring process shall より適切な名前へのリネームを検討する
3. The volcube module shall `proptest_tests.rs`や`aad_validation.rs`等のテスト・検証用コードを適切な場所に配置する
4. The volcube module shall `loader_convert.rs`等のユーティリティ機能の位置づけを明確化する
5. The volcube module shall mod.rsのre-exportを整理し、内部実装と公開APIを明確に分離する

### Requirement 4: surfaces/とcurves/の構造統一

**Objective:** As a 開発者, I want surfaces/とcurves/の内部構造を統一する, so that 類似概念（trait、flat実装、interpolated実装、enum）の配置が一貫する

#### Acceptance Criteria
1. The market module shall curves/とsurfaces/で同様のファイル命名規則を採用する（traits.rs、flat.rs、interpolated.rs、enum.rs等）
2. When 両モジュールで異なるパターンが使われている場合, the refactoring process shall より良いパターンに統一する
3. The market module shall `volcube_slice.rs`と`vol_surface_enum.rs`の配置を再評価し、surfaces/内の論理的な位置に配置する
4. The market module shall CurveEnumとVolSurfaceEnumのコード構造を統一し、将来の拡張を容易にする

### Requirement 5: ルートレベルファイルの整理

**Objective:** As a 開発者, I want market/直下のルートレベルファイルを整理する, so that モジュールのエントリポイントが明確になる

#### Acceptance Criteria
1. The market module shall `index_mapper.rs`と`indexed_market.rs`の関係を明確化し、必要に応じて統合または適切なサブモジュールへの移動を行う
2. The market module shall `provider.rs`、`requirements.rs`、`validator.rs`の役割と配置を再評価する
3. When 機能が単一ファイルに収まらない規模に成長している場合, the refactoring process shall サブモジュール化を検討する
4. The market module shall `fx_density.rs`が`fx_calibration/`に属すべきか評価し、適切に配置する
5. The market module shall mod.rsのre-exportを整理し、公開APIの明確なカテゴリ分けを提供する

### Requirement 6: 後方互換性の維持

**Objective:** As a 開発者, I want リファクタリング中も後方互換性を維持する, so that 既存の利用コードが壊れない

#### Acceptance Criteria
1. The pricer_models shall 既存の公開API（`pricer_models::market::*`）をdeprecation警告付きで維持するか、または完全な移行パスを提供する
2. When ファイルパスが変更される場合, the refactoring process shall 旧パスからの再エクスポートを一時的に提供する
3. The pricer_models shall 変更後もすべての既存テストがパスすることを確認する
4. The pricer_models shall `cargo doc`による文書生成が正常に動作することを確認する
5. If 破壊的変更が必要な場合, then the refactoring process shall CHANGELOGに明確に記載する

### Requirement 7: ドキュメントとモジュール構造の同期

**Objective:** As a 開発者, I want steering/structure.mdとコード構造を同期する, so that ドキュメントと実装の乖離がなくなる

#### Acceptance Criteria
1. The pricer_models shall リファクタリング完了後、`steering/structure.md`のmarket/セクションを更新する
2. The market module shall 各サブモジュールのmod.rsに適切なモジュールドキュメント（`//!`コメント）を追加・更新する
3. The market module shall 新しいファイル構造を反映したディレクトリツリーを文書化する
4. When 機能が移動された場合, the refactoring process shall ドキュメント内の参照パスも更新する
