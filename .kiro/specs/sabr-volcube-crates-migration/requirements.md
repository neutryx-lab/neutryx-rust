# Requirements Document

## Introduction

本仕様は、demo/gui内に存在するSABR VolCubeカリブレーションのスタンドアロン実装を`pricer_models`に完全移行することを目的とする。

### 背景
現在の実装には以下のアーキテクチャ違反がある：
- **コード重複**: `pricer_core`に同等のHagan公式とLMソルバーが既に存在
- **レイヤー違反**: ビジネスロジックがL5（demo）に存在すべきでない（A-I-P-S原則違反）
- **テスト困難**: UIハンドラ内にロジックが埋め込まれている
- **再利用不可**: 他のサービス（Python binding等）から使用できない

### 目標
全てのSABRカリブレーションロジックを`pricer_models::builder::vol`に集約し、demo/guiは単にAPIエンドポイントとしてcratesを呼び出すのみとする。

### 既存の実装（削除対象）
ファイル: `demo/gui/src/web/handlers/volcube.rs`

以下の関数がスタンドアロンで実装されている：
- `calibrate_sabr_simple()` - Levenberg-Marquardt最適化（約80行）
- `optimize_sabr()` - LMアルゴリズム本体
- `sabr_implied_vol()` - Hagan公式（SABR implied vol）
- `black_call_price()` - Black公式（価格計算用）

### 既存のcrates実装（活用対象）
- `pricer_core::math::formulas::sabr` - 完全なHagan公式実装
- `pricer_core::math::solvers::levenberg_marquardt` - 汎用LMソルバー
- `pricer_models::builder::vol` - VolCubeBuilder（構造のみ、TODO実装）
- `infra_master::trade::instrument_def::rates` - Swaption定義
- `infra_master::trade::convention::swaption` - SwaptionConvention

---

## Requirements

### Requirement 1: VolQuoteデータ構造の拡張

**Objective:** As a カリブレーションエンジン開発者, I want VolQuoteにexpiry情報を含める, so that SABRカリブレーションで正確なHagan公式計算が可能になる

#### Acceptance Criteria
1. The `VolQuote<T>` shall `expiry: T`フィールドを持つ
2. When `VolQuote::new()`が呼び出された場合, the `VolQuote` shall strike, volatility, forward, expiryの4パラメータを受け取る
3. The `VolQuote` shall 後方互換性のため`new_without_expiry()`コンストラクタを提供し、expiryをT::one()にデフォルト設定する
4. When expiryフィールドが追加された場合, the 既存テスト shall 新しいシグネチャに更新される

---

### Requirement 2: SliceCalibrationConfigの拡張

**Objective:** As a カリブレーションエンジン開発者, I want カリブレーション設定を拡張する, so that LMソルバーの詳細な制御とパラメータ境界制約が可能になる

#### Acceptance Criteria
1. The `SliceCalibrationConfig<T>` shall 以下の追加フィールドを持つ：`initial_rho`, `initial_nu`, `lm_lambda`, `lm_lambda_factor`, `bounds`
2. The `SabrBounds<T>` shall alpha, rho, nuの最小値と最大値を定義する構造体として実装される
3. When `SabrBounds::default()`が呼び出された場合, the SabrBounds shall 以下のデフォルト値を返す：alpha_min=1e-6, alpha_max=1.0, rho_min=-0.99, rho_max=0.99, nu_min=1e-6, nu_max=2.0
4. When `SliceCalibrationConfig::default()`が呼び出された場合, the SliceCalibrationConfig shall initial_rho=-0.3, initial_nu=0.4, lm_lambda=0.001, lm_lambda_factor=10.0を返す
5. The `SliceCalibrationConfig::rates()` shall β=0.5に適した金利スワプション用プリセットを返す
6. The `SliceCalibrationConfig::fx()` shall β=1.0に適したFXオプション用プリセットを返す

---

### Requirement 3: SABR残差関数の実装（クロージャベースAPI）

**Objective:** As a カリブレーションエンジン開発者, I want LMソルバー用の残差関数を実装する, so that pricer_coreのLevenbergMarquardtSolverと統合できる

#### Acceptance Criteria
1. The `SabrSliceCalibrator` shall `pricer_core::math::solvers::LevenbergMarquardtSolver`のクロージャベースAPI `Fn(&[f64]) -> Vec<f64>` を使用する
2. The 残差クロージャ shall quotes, beta, forward, expiryをキャプチャする
3. When 残差関数が呼び出された場合, the クロージャ shall 3パラメータ（alpha, rho, nu）を受け取る
4. When 残差関数が呼び出された場合, the クロージャ shall クォート数と同じ長さのVec<f64>を返す
5. When 残差が計算される場合, the クロージャ shall 各クォートに対して`σ_market - σ_sabr`を計算する
6. The ヤコビアン shall `LevenbergMarquardtSolver`内部の有限差分法で自動計算される
7. The 残差計算 shall `pricer_core::math::formulas::sabr::sabr_implied_vol()`を使用してモデルボラティリティを計算する

---

### Requirement 4: SabrSliceCalibratorの完全実装

**Objective:** As a カリブレーションエンジン開発者, I want SabrSliceCalibratorのTODOプレースホルダーを完全な実装に置き換える, so that 実際のSABRパラメータカリブレーションが可能になる

#### Acceptance Criteria
1. When `calibrate_slice()`が呼び出された場合, the SabrSliceCalibrator shall `pricer_core::math::solvers::LevenbergMarquardtSolver`を使用して最適化を実行する
2. When quotesが空の場合, the SabrSliceCalibrator shall `CalibrationError::InsufficientData`を返す
3. When カリブレーションが収束しない場合, the SabrSliceCalibrator shall `CalibrationError::NonConvergence`を返す
4. When 最適化が成功した場合, the SabrSliceCalibrator shall 検証済みの`SabrParams<T>`を返す
5. The SabrSliceCalibrator shall ATMクォートから初期alphaを推定する（α ≈ σ_ATM × F^(1-β)）
6. The SabrSliceCalibrator shall config.boundsを使用してパラメータ境界を適用する
7. When カリブレーション結果が得られた場合, the SabrSliceCalibrator shall `SabrParams::validate()`を呼び出して検証する

---

### Requirement 5: VolCubeBuilder APIの更新

**Objective:** As a VolCubeユーザー, I want add_quote()メソッドを使用してexpiryを指定する, so that 3次元VolCubeを正しく構築できる

#### Acceptance Criteria
1. When `add_quote()`が呼び出された場合, the VolCubeBuilder shall expiry, tenor, strike, volatility, forwardの5パラメータを受け取る
2. When `add_slice()`が呼び出された場合, the VolCubeBuilder shall expiry, tenorと共にVec<VolQuote<T>>を受け取る
3. When `calibrate()`が呼び出された場合, the VolCubeBuilder shall 各(expiry, tenor)スライスに対してSabrSliceCalibratorを使用する
4. When カリブレーションが成功した場合, the VolCubeBuilder shall 有効なSabrParamsを持つVolCubeResultを返す

---

### Requirement 6: 既存CalibrationErrorの活用

**Objective:** As a エラーハンドリング開発者, I want 既存のCalibrationErrorバリアントを活用する, so that カリブレーション失敗の原因を明確に伝えられる

#### Acceptance Criteria
1. The `SabrSliceCalibrator` shall 既存の`CalibrationError::ConvergenceFailure { iterations, residual }`を非収束時に使用する
2. The `SabrSliceCalibrator` shall 既存の`CalibrationError::NumericalInstability { message }`をソルバーエラー時に使用する
3. When LMソルバーがエラーを返した場合, the SabrSliceCalibrator shall `NumericalInstability`にラップして返す
4. When 最大イテレーション内で収束しない場合, the SabrSliceCalibrator shall `ConvergenceFailure`を返す

---

### Requirement 7: demo/guiリファクタリング

**Objective:** As a アーキテクチャ管理者, I want demo/guiからスタンドアロン関数を削除する, so that A-I-P-S原則に準拠したクリーンなレイヤー分離が実現する

#### Acceptance Criteria
1. When demo/guiがリファクタリングされた場合, the volcube.rs shall `calibrate_sabr_simple()`, `optimize_sabr()`, `sabr_implied_vol()`, `black_call_price()`を削除する
2. When VolCubeカリブレーションAPIが呼び出された場合, the volcube_handlers shall `pricer_models::builder::vol::VolCubeBuilder`を使用する
3. When APIリクエストを処理する場合, the volcube_handlers shall `SliceCalibrationConfig`をリクエストパラメータから構築する
4. When カリブレーションエラーが発生した場合, the volcube_handlers shall `CalibrationError`を適切なHTTPレスポンスに変換する
5. The demo/gui shall pricer_models crateへの依存を追加する

---

### Requirement 8: 単体テストと検証

**Objective:** As a 品質管理者, I want 包括的なテストカバレッジを確保する, so that カリブレーションロジックの正確性が検証される

#### Acceptance Criteria
1. The テストスイート shall ATMボラティリティから正しくalphaを推定できることを検証する
2. The テストスイート shall スマイルデータに対してカリブレーションが収束することを検証する
3. The テストスイート shall カリブレーション後のモデルvolが市場volに対して50bp未満の誤差であることを検証する
4. The テストスイート shall 空のquotesに対して`InsufficientData`エラーを返すことを検証する
5. The テストスイート shall 複数スライスのVolCubeカリブレーションが正しく動作することを検証する
6. When `cargo test -p pricer_models`が実行された場合, the 全てのテスト shall パスする
7. The カリブレーション結果 shall α>0, -1<ρ<1, ν>0の制約を満たす

---

### Requirement 9: 性能と収束性

**Objective:** As a カリブレーションユーザー, I want 合理的な時間内に収束する, so that 実用的なパフォーマンスが得られる

#### Acceptance Criteria
1. The SabrSliceCalibrator shall デフォルトで100イテレーション以内に収束する
2. The SabrSliceCalibrator shall tolerance=1e-8をデフォルト収束基準とする
3. When 典型的なスワプションスマイルデータが与えられた場合, the カリブレーション shall 50イテレーション以内に収束する
4. The カリブレーション結果 shall 市場データに対して再現可能である（同じ入力で同じ出力）

---

## Technical Constraints

### 依存関係
- `pricer_models` → `pricer_core`（LMソルバー、SABR公式）
- `demo/gui` → `pricer_models`（VolCubeBuilder）

### アーキテクチャ準拠
- ビジネスロジックはPricerレイヤー（L2: pricer_models）に配置
- demo/guiはService/Demoレイヤーとしてcratesを呼び出すのみ
- 静的ディスパッチ（enum）を優先（Enzyme互換性）

### 命名規則
- British English: `optimiser`, `calibrator`, `behaviour`
- snake_case for modules and functions
- PascalCase for types and traits
