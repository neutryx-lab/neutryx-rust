# Requirements Document

## Introduction

本仕様は、`infra_master`クレート内にCounterParty（取引相手先）とネッティングセット情報を管理するための専用モジュール構造を構築することを目的とする。現在`counterparty.rs`に定義されている`CsaTerms`と`NettingSetConfig`を新しい`counterparty/`フォルダに移行・拡張し、Tier-1銀行の本番運用に必要なXVA計算・担保管理・SIMM計算・Exposure管理機能を支援する包括的なカウンターパーティ管理基盤を提供する。

本モジュールは**静的マスターデータ**の定義に特化し、計算ロジック（XVA計算、Exposure計算等）は`pricer_risk`クレートに委譲する。A-I-P-S依存規則に従い、`infra_master`は他のPricerクレートに依存しない。

## Requirements

### Requirement 1: モジュール構造の再編成

**Objective:** As a 開発者, I want CounterPartyとネッティングセット関連の型を専用フォルダで管理したい, so that コードの保守性と拡張性が向上し、関連機能の追加が容易になる

#### Acceptance Criteria
1. The infra_master shall 新規フォルダ`crates/infra_master/src/counterparty/`を作成する
2. The infra_master shall `mod.rs`、`csa.rs`、`netting_set.rs`、`counterparty.rs`、`credit.rs`、`margin.rs`、`error.rs`の7ファイル構成とする
3. The infra_master shall 既存の`CsaTerms`と`NettingSetConfig`を新モジュールに移行する
4. The infra_master shall クレートルートからの再エクスポートを維持し、後方互換性を確保する
5. The infra_master shall 新モジュール用のpreludeエクスポートを追加する

### Requirement 2: CounterParty型の新規定義

**Objective:** As a リスク管理者, I want 取引相手先の情報を構造化して管理したい, so that XVA計算やクレジットリスク評価に必要な情報を一元管理できる

#### Acceptance Criteria
1. The infra_master shall `CounterParty`構造体を定義し、`counterparty_id`、`name`、`lei`、`sector`、`country`、`rating`フィールドを含める
2. The infra_master shall `CounterPartyId`型（新型パターン）を定義し、型安全なID参照を提供する
3. When CounterPartyが作成されるとき, the infra_master shall ビルダーパターンによる構築をサポートする
4. Where serde featureが有効な場合, the infra_master shall CounterPartyのシリアライズ/デシリアライズをサポートする
5. The infra_master shall `CounterPartySector` enumを定義し、Banking、Investment、Securities、Insurance、Trading、AssetManagement、HedgeFund、Corporate、Sovereign、Otherを含める
6. The infra_master shall `LegalEntityId`型（LEI: Legal Entity Identifier）を定義し、ISO 17442準拠の20文字IDをサポートする

### Requirement 3: クレジットパラメータ

**Objective:** As a XVA計算担当者, I want 取引相手先のクレジットパラメータを管理したい, so that CVA/DVA計算に必要なデフォルト確率とLGDを一元管理できる

#### Acceptance Criteria
1. The infra_master shall `CreditRating` enumを定義し、主要格付け（AAA、AA+、AA、AA-、A+、A、A-、BBB+、BBB、BBB-、BB+、BB、BB-、B+、B、B-、CCC、CC、C、D）をサポートする
2. The infra_master shall `CreditParams`構造体を定義し、`hazard_rate`、`lgd`、`recovery_rate`、`pd_1y`（1年デフォルト確率）フィールドを含める
3. The infra_master shall CreditRatingから指標的なhazard_rateを取得する`indicative_hazard_rate()`メソッドを提供する
4. The infra_master shall CreditRatingに`is_investment_grade()`メソッドを提供する（BBB-以上がtrue）
5. When CreditParamsが作成されるとき, the infra_master shall hazard_rateとpd_1yの相互変換をサポートする

### Requirement 4: NettingSet型の拡張

**Objective:** As a XVA計算担当者, I want ネッティングセットに関連する全ての情報にアクセスしたい, so that 正確なエクスポージャー計算とXVA評価が可能になる

#### Acceptance Criteria
1. The infra_master shall `NettingSet`構造体を定義し、`netting_set_id`、`counterparty_id`、`legal_entity_id`、`csa_terms`、`margin_terms`を含める
2. The infra_master shall `NettingSetId`型（新型パターン）を定義し、型安全なID参照を提供する
3. When NettingSetが作成されるとき, the infra_master shall 関連するCounterPartyIdへの参照を保持する
4. The infra_master shall ネッティングセットの種類を`NettingType` enumで区別する（Bilateral、ClearedCcp、ClearedClient）
5. The infra_master shall クローズアウトネッティング適用フラグ（`closeout_netting: bool`）を含める
6. If 必須フィールドが欠落している場合, the infra_master shall `CounterPartyError`を返す

### Requirement 5: CSA条件の拡張

**Objective:** As a コラテラル管理担当者, I want CSA条件を詳細に設定したい, so that 各取引相手先との契約条件を正確に反映できる

#### Acceptance Criteria
1. The infra_master shall 既存の`CsaTerms`を新モジュールに移行し、後方互換性を維持する
2. The infra_master shall 適格担保の種類を`EligibleCollateral` enumで定義する（Cash、GovernmentBonds、CorporateBonds、Equity、Gold）
3. The infra_master shall `CollateralHaircut`構造体を定義し、担保種別・通貨ごとのヘアカット率を設定可能とする
4. The infra_master shall リハイポ可否（`rehypothecation: bool`）フィールドを追加する
5. The infra_master shall 担保分別管理種別を`SegregationType` enumで定義する（Segregated、Commingled）
6. When CSA条件が適用されるとき, the infra_master shall 通貨別の閾値設定（`HashMap<Currency, f64>`）をサポートする
7. The infra_master shall マージンコール頻度を`CallFrequency` enumで定義する（Daily、Weekly、Monthly）
8. The infra_master shall 係争閾値（`dispute_threshold: f64`）フィールドを追加する

### Requirement 6: VM/IM マージン条件

**Objective:** As a コラテラル管理担当者, I want Variation MarginとInitial Marginの詳細条件を管理したい, so that UMR規制に準拠したマージン運用が可能になる

#### Acceptance Criteria
1. The infra_master shall `MarginTerms`構造体を定義し、VM条件とIM条件を統合管理する
2. The infra_master shall `MarginType` enumを定義する（NoMargin、VmOnly、VmAndIm）
3. The infra_master shall `VmTerms`構造体を定義し、`frequency`、`settlement_lag`、`rounding`フィールドを含める
4. The infra_master shall `ImTerms`構造体を定義し、`model`、`calculation_frequency`、`posting_currency`フィールドを含める
5. The infra_master shall `ImModel` enumを定義する（SIMM、Schedule、Grid、Internal）
6. The infra_master shall `SimmVersion` enumを定義し、主要バージョン（V2_5、V2_6、V2_7）をサポートする
7. When SIMM計算が必要なとき, the infra_master shall リスククラス対応情報（`SimmRiskClassMapping`）を提供する
8. The infra_master shall `RoundingRule`構造体を定義し、丸め金額と丸め方向を設定可能とする

### Requirement 7: Exposure管理パラメータ

**Objective:** As a リスク管理者, I want Exposure計算に必要なパラメータを管理したい, so that EE/EPE/PFE/EEPE計算の設定を一元管理できる

#### Acceptance Criteria
1. The infra_master shall `ExposureConfig`構造体を定義し、エクスポージャー計算のパラメータを含める
2. The infra_master shall シミュレーション時間グリッド設定（`time_grid_years: Vec<f64>`）をサポートする
3. The infra_master shall PFE信頼水準設定（`pfe_confidence: f64`、デフォルト0.95）をサポートする
4. The infra_master shall 規制EEPE計算のmaturity設定（`regulatory_maturity: f64`、デフォルト1.0年）をサポートする
5. The infra_master shall ネッティング適用フラグ（`apply_netting: bool`）を含める
6. The infra_master shall 担保効果適用フラグ（`apply_collateral: bool`）を含める

### Requirement 8: CCP（中央清算機関）情報

**Objective:** As a リスク管理者, I want CCP経由の取引に必要な情報を管理したい, so that 清算取引の特性を正確に反映できる

#### Acceptance Criteria
1. The infra_master shall `CcpId`型（新型パターン）を定義し、CCPの識別子を提供する
2. The infra_master shall `Ccp`構造体を定義し、`ccp_id`、`name`、`country`、`qualifying`フィールドを含める
3. The infra_master shall 適格CCP判定フラグ（`qualifying: bool`）をサポートする（SA-CCR用）
4. When NettingTypeがClearedの場合, the infra_master shall CcpIdへの参照をオプションで保持する
5. The infra_master shall CCP固有のマージン期間（cleared MPOR: 5営業日）をデフォルト値として提供する

### Requirement 9: エラーハンドリング

**Objective:** As a 開発者, I want CounterPartyモジュール固有のエラー型を使用したい, so that エラーの原因を特定しやすくなる

#### Acceptance Criteria
1. The infra_master shall `CounterPartyError` enumを定義する
2. The infra_master shall `InvalidCounterPartyId`、`InvalidNettingSetId`、`InvalidLei`、`MissingCsaTerms`、`InvalidRating`、`InvalidCreditParams`、`InvalidMarginTerms`、`InvalidHaircut`バリアントを含める
3. The infra_master shall `thiserror`を使用してエラーメッセージを実装する
4. The infra_master shall `std::error::Error`トレイトを実装する
5. The infra_master shall 既存の`MasterDataError`との変換（From実装）を提供する

### Requirement 10: 型安全なID参照

**Objective:** As a 開発者, I want 各種IDを型安全に扱いたい, so that 異なるID型の混同を防止できる

#### Acceptance Criteria
1. The infra_master shall `CounterPartyId`、`NettingSetId`、`LegalEntityId`、`CcpId`、`TradeId`を新型パターンで実装する
2. The infra_master shall 全ID型に`Display`、`Debug`、`Clone`、`PartialEq`、`Eq`、`Hash`トレイトを実装する
3. The infra_master shall 全ID型に`AsRef<str>`と`From<String>`を実装する
4. Where serde featureが有効な場合, the infra_master shall ID型のシリアライズをtransparentに行う
5. The infra_master shall `LegalEntityId`にISO 17442バリデーション（20文字英数字）を提供する
6. The infra_master shall `pricer_risk::portfolio::ids`の型を`infra_master`からの再エクスポートに移行可能な設計とする

### Requirement 11: pricer_riskとの統合設計

**Objective:** As a アーキテクト, I want infra_masterとpricer_riskの型を整理したい, so that A-I-P-S依存規則に従った一貫性のあるアーキテクチャを実現できる

#### Acceptance Criteria
1. The infra_master shall `CreditRating`を定義し、`pricer_risk::portfolio::CreditRating`と同等の機能を提供する
2. The infra_master shall `CreditParams`を定義し、`hazard_rate`、`lgd`、`pd_1y`を含め、`survival_prob()`、`default_prob()`、`marginal_default_prob()`メソッドを提供する
3. The infra_master shall `CounterParty`を定義し、`pricer_risk::portfolio::Counterparty`の静的マスターデータ部分を包含する
4. When pricer_riskがCounterParty情報を必要とするとき, the pricer_risk shall infra_master::counterpartyモジュールの型を使用する
5. The infra_master shall `CollateralAgreement`の静的パラメータ（threshold, mta, independent_amount, mpor）を`CsaTerms`で提供する
6. The infra_master shall 後方互換性のため、移行期間中は`pricer_risk`での型エイリアスを推奨する

---

## Appendix: 用語定義

| 用語 | 説明 |
|------|------|
| **VM (Variation Margin)** | 時価変動に基づく日次担保授受 |
| **IM (Initial Margin)** | 将来のポテンシャルエクスポージャーをカバーする当初証拠金 |
| **SIMM** | ISDA Standard Initial Margin Model（規制IM計算標準モデル） |
| **MPOR** | Margin Period of Risk（マージン期間リスク）、清算=5日、相対=10日以上 |
| **CSA** | Credit Support Annex（ISDA担保契約の付属書） |
| **CCP** | Central Counterparty Clearing House（中央清算機関） |
| **UMR** | Uncleared Margin Rules（非清算デリバティブの証拠金規制） |
| **LEI** | Legal Entity Identifier（取引主体識別子、ISO 17442） |
| **LGD** | Loss Given Default（デフォルト時損失率） |
| **EE/EPE/PFE/EEPE** | Expected Exposure / Expected Positive Exposure / Potential Future Exposure / Effective EPE |