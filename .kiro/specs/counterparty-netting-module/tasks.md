# Implementation Plan

## Task 1: モジュール基盤構築とエラー型定義

- [x] 1.1 CounterPartyモジュール専用エラー型を定義する
  - thiserrorを使用したCounterPartyError enumの実装
  - 全バリアント（InvalidCounterPartyId, InvalidNettingSetId, InvalidLei, MissingCsaTerms, InvalidRating, InvalidCreditParams, InvalidMarginTerms, InvalidHaircut）の定義
  - std::error::Errorトレイトの自動実装
  - MasterDataErrorとのFrom変換実装
  - _Requirements: 9_

- [x] 1.2 counterpartyモジュールの骨格を作成し既存型を移行する
  - counterparty/フォルダとmod.rsの作成
  - 既存CsaTermsとNettingSetConfigの新モジュールへの移行
  - サブモジュール宣言（csa, netting_set, counterparty, credit, margin, error）
  - 公開型の再エクスポート設定
  - preludeモジュールの定義
  - クレートルート（lib.rs）からの後方互換エクスポート維持
  - _Requirements: 1_

## Task 2: 型安全なID型の実装

- [x] 2.1 (P) CounterPartyIdとLegalEntityIdを新型パターンで実装する
  - CounterPartyId構造体（String内部表現）の定義
  - Display, Debug, Clone, PartialEq, Eq, Hashトレイト実装
  - AsRef<str>とFrom<String>、From<&str>実装
  - feature-gated serde（transparent）サポート
  - LegalEntityId構造体とISO 17442バリデーション（20文字英数字）
  - new_unchecked()メソッド（信頼できるソース用）
  - _Requirements: 2, 10_

- [x] 2.2 (P) NettingSetIdとCcpIdを新型パターンで実装する
  - NettingSetId構造体の定義
  - CcpId構造体の定義
  - 全ID型に共通のトレイト実装（Display, Debug, Clone, PartialEq, Eq, Hash）
  - AsRef<str>とFrom<String>、From<&str>実装
  - feature-gated serde（transparent）サポート
  - _Requirements: 4, 8, 10_

## Task 3: クレジットパラメータの実装

- [x] 3.1 20段階CreditRating enumを定義する
  - Aaa〜D（+/-ノッチ含む20バリアント）の定義
  - PartialOrd, Ordトレイト実装（格付け順序）
  - is_investment_grade()メソッド（BbbMinus以上がtrue）
  - indicative_hazard_rate()メソッド（各格付けの参考ハザードレート）
  - feature-gated serde（文字列表現）
  - _Requirements: 3_

- [x] 3.2 CreditParams構造体を実装する
  - hazard_rate、lgd、pd_1y、ratingフィールドの定義
  - new()コンストラクタ（バリデーション付き）
  - from_rating()ファクトリメソッド
  - from_pd_1y()ファクトリメソッド（pd_1y → hazard_rate変換）
  - survival_prob(t)、default_prob(t)、marginal_default_prob(t1, t2)メソッド
  - recovery_rate()アクセサ（1 - lgd）
  - pricer_risk::CreditParamsと同等のAPI互換性
  - _Requirements: 3, 11_

## Task 4: CSA条件の拡張実装

- [x] 4.1 (P) 担保関連のenum型を定義する
  - EligibleCollateral enum（Cash, GovernmentBonds, CorporateBonds, Equity, Gold）
  - SegregationType enum（Segregated, Commingled）
  - CallFrequency enum（Daily, Weekly, Monthly）
  - 全enumにfeature-gated serde実装
  - _Requirements: 5_

- [x] 4.2 CollateralHaircutとCsaTermsを実装する
  - CollateralHaircut構造体（担保種別、通貨、ヘアカット率）
  - ヘアカット率のバリデーション（0-1範囲）
  - CsaTerms構造体の拡張（既存フィールド + 新フィールド）
  - threshold、mta、independent_amount、mpor_days、margin_currency
  - currency_thresholds（HashMap<Currency, f64>）
  - eligible_collateral、haircuts、rehypothecation、segregation
  - call_frequency、dispute_threshold
  - threshold_for_currency()メソッド（通貨別閾値取得）
  - CsaTermsBuilderによるビルダーパターン実装
  - _Requirements: 5, 11_

## Task 5: マージン条件の実装

- [x] 5.1 (P) マージン関連のenum型を定義する
  - MarginType enum（NoMargin, VmOnly, VmAndIm）
  - ImModel enum（Simm, Schedule, Grid, Internal）
  - SimmVersion enum（V2_5, V2_6, V2_7）
  - RoundingDirection enum（Nearest, Up, Down）
  - 全enumにfeature-gated serde実装
  - _Requirements: 6_

- [x] 5.2 VM/IM条件構造体を実装する
  - RoundingRule構造体（丸め金額、丸め方向）
  - apply()メソッドによる丸め計算
  - VmTerms構造体（frequency, settlement_lag, rounding）
  - ImTerms構造体（model, simm_version, calculation_frequency, posting_currency）
  - SimmRiskClassMapping構造体（プレースホルダー）
  - MarginTerms構造体（margin_type, vm_terms, im_terms）
  - no_margin()、vm_only()、vm_and_im()ファクトリメソッド
  - _Requirements: 6_

## Task 6: CounterPartyとCCPエンティティの実装

- [x] 6.1 CounterPartySector enumとCounterParty構造体を実装する
  - CounterPartySector enum（10業種）
  - CounterParty構造体の定義（id, name, lei, sector, country, rating, credit_params）
  - CounterPartyBuilderによるビルダーパターン実装
  - 全アクセサメソッドの実装
  - feature-gated serdeサポート
  - _Requirements: 2_

- [x] 6.2 Ccp（中央清算機関）構造体を実装する
  - Ccp構造体（ccp_id, name, country, qualifying）
  - CLEARED_MPOR_DAYS定数（5営業日）
  - is_qualifying()メソッド（SA-CCR用適格判定）
  - newコンストラクタとwith_countryメソッド
  - feature-gated serdeサポート
  - _Requirements: 8_

## Task 7: NettingSetとExposureConfigの実装

- [x] 7.1 NettingType enumとExposureConfigを実装する
  - NettingType enum（Bilateral, ClearedCcp, ClearedClient）
  - ExposureConfig構造体（time_grid_years, pfe_confidence, regulatory_maturity, apply_netting, apply_collateral）
  - デフォルト値の設定（pfe_confidence: 0.95, regulatory_maturity: 1.0）
  - ビルダースタイルのwith_*メソッド
  - _Requirements: 4, 7_

- [x] 7.2 NettingSet構造体とビルダーを実装する
  - NettingSet構造体（全フィールド：id, counterparty_id, legal_entity_id, netting_type, closeout_netting, csa_terms, margin_terms, ccp_id, exposure_config）
  - NettingSetBuilderによるビルダーパターン実装
  - build()でのバリデーション（必須フィールドチェック）
  - mpor_days()メソッド（CCP/Bilateral判定によるMPOR取得）
  - CounterPartyIdへの必須参照維持
  - _Requirements: 4, 7_

## Task 8: 統合とテスト

- [x] 8.1 モジュール統合と後方互換性の確認
  - mod.rsでの全型再エクスポート確認
  - preludeの完成（全主要型を含む）
  - lib.rsでの後方互換エクスポート（既存パスからのアクセス維持）
  - infra_master::CsaTerms、infra_master::NettingSetConfigの既存パス維持
  - cargo check --all-featuresによるコンパイル確認
  - _Requirements: 1, 11_

- [x] 8.2 単体テストを実装する
  - CreditRating::is_investment_grade()境界値テスト
  - CreditParams::survival_prob()数学的正確性テスト
  - LegalEntityId::new()バリデーションテスト（正常系・異常系）
  - NettingSet::mpor_days()テスト（CCP=5日、Bilateral=CSAまたは10日）
  - CollateralHaircut範囲検証テスト
  - RoundingRule::apply()テスト（各丸め方向）
  - CounterPartyBuilder、NettingSetBuilderテスト
  - _Requirements: 3, 4, 9, 10_

- [x] 8.3 serde機能の統合テストを実装する
  - 全主要型のJSON往復シリアライズテスト
  - ID型のtransparentシリアライズ確認
  - CreditRating文字列表現テスト
  - _Requirements: 2, 3, 10_
