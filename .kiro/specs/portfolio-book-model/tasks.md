# Implementation Plan

## Task Overview

本実装計画は`infra_master`クレートにPortfolio/Book定義およびCounterpartyPortfolio階層構造を追加する。XVA計算、Exposure計算、Netting計算の基盤構造を提供する。

---

## Tasks

- [x] 1. 基盤ID型とエラー型の定義
- [x] 1.1 (P) 新規ID型の定義
  - IsdaAgreementId, VariationMarginAgreementIdをdefine_id!マクロで定義
  - 既存パターン（CounterPartyId, NettingSetId）に準拠したderive属性
  - serde feature flagによる条件付きシリアライゼーション
  - _Requirements: 12.1, 13.1_

- [x] 1.2 (P) Book/Portfolio関連エラー型の定義
  - BookError列挙型（DuplicateId, InvalidOwnership, InvalidType, MissingRequiredField）
  - PortfolioError列挙型（DuplicateId, CircularReference, InvalidBookReference, InvalidScope）
  - thiserror deriveによるDisplay実装
  - _Requirements: 9.1, 9.2_

- [x] 1.3 (P) Netting/Exposure関連エラー型の定義
  - NettingError列挙型（CounterpartyMismatch, NotEnforceable, InvalidAgreement, CrossBookViolation）
  - ExposureError列挙型（MissingDate, CurrencyMismatch, InvalidTimeGrid）
  - ValidationError統合型とFrom実装
  - ValidationResult型エイリアス定義
  - _Requirements: 9.3, 9.4, 9.5, 9.6, 9.7_

---

- [x] 2. Book概念の実装
- [x] 2.1 Book構造体と関連型の定義
  - BookType列挙型（Trading, Banking, Hedge, Internal）とデフォルト値
  - RegulatoryBookType列挙型（TB, NTBR, BB）
  - BookOwnership構造体（desk, division, legal_entity_id）
  - BookMetadata構造体（タイムスタンプ、監査情報）
  - Book構造体の全フィールド定義
  - _Requirements: 1.1, 1.2, 1.3, 1.7, 1.8_

- [x] 2.2 BookBuilderの実装
  - fluent APIによるBookインスタンス構築
  - 必須フィールド（id, name）のコンストラクタ引数
  - オプションフィールドのbuilderメソッド
  - BookType::Tradingへのデフォルト適用
  - build()によるバリデーションとBook返却
  - _Requirements: 1.4, 1.5, 1.6_

---

- [x] 3. Portfolio定義の実装
- [x] 3.1 (P) Portfolio構造体と関連型の定義
  - PortfolioScope列挙型（Internal, Legal, Regulatory, Consolidated）
  - PortfolioMetadata構造体（ownership, scope, reporting_currency, timestamps）
  - PortfolioBookMapping構造体（portfolio_id, book_id, weight）
  - PortfolioDefinition構造体の全フィールド定義
  - _Requirements: 2.1, 2.2, 2.7, 2.8_

- [x] 3.2 (P) PortfolioBuilderの実装
  - fluent APIによるPortfolioインスタンス構築
  - Book参照の整合性バリデーション
  - 親ポートフォリオ階層のサポート
  - 循環参照検出ロジック
  - _Requirements: 2.3, 2.4, 2.5, 2.6_

---

- [x] 4. Trade-Book関連付けの実装
- [x] 4.1 TradeMetadataの更新
  - book_idフィールドはOption<BookId>のまま維持（後方互換性のため）
  - TradeBookAssignment/TradeBookHistoryで必須book_id管理を提供
  - _Requirements: 3.1, 3.2, 11.2, 11.4, 11.5, 11.6_

- [x] 4.2 (P) TradeBookAssignmentの実装
  - BookTransferReason列挙型（NewTrade, Reallocation, Novation, InternalTransfer）
  - TradeBookAssignment構造体（trade_id, book_id, effective_date, reason, previous_book_id）
  - TradeBookHistory構造体によるBook移管履歴の追跡機能
  - _Requirements: 3.3, 3.4, 3.5, 3.6_

---

- [x] 5. Book-NettingSet関係の実装
- [x] 5.1 NettingSetへのbook_id参照追加
  - 既存NettingSet構造体にbook_idsフィールド追加（Vec<BookId>）
  - allows_cross_book_netting(), allows_book()メソッド
  - NettingSetBuilder::add_book(), book_ids()メソッド
  - _Requirements: 4.1, 4.2, 4.4, 4.6, 4.7_

- [x] 5.2 (P) CrossBookNettingAgreementの実装
  - 複数Book間ネッティングの設定構造
  - cross-bookネッティング時の明示的合意要件
  - _Requirements: 4.3, 4.5_

---

- [x] 6. VariationMarginAgreement（非対称条件）の実装
- [x] 6.1 担保コール頻度とMPOR決定の実装
  - CollateralCallFrequency列挙型（Daily, Weekly, Biweekly, Monthly）
  - default_mpor_days()メソッドによるMPOR値返却
  - _Requirements: 13.6, 17.1, 17.2, 17.3, 17.4, 17.5, 17.6_

- [x] 6.2 非対称担保条件の実装
  - threshold_cpty/threshold_self（非対称threshold）
  - mta_cpty/mta_self（非対称MTA）
  - haircut_cpty/haircut_self（非対称haircut）
  - IndependentAmountConfig構造体（ia_cpty, k_cpty, ia_self, k_self）
  - 動的IA計算ロジック（calculate()メソッド）
  - _Requirements: 13.2, 13.3, 13.4, 13.5_

- [x] 6.3 VariationMarginAgreement構造体の実装
  - VariationMarginAgreementIdによる一意識別
  - 基本属性（name, base_currency, call_frequency）
  - 適格担保と現在残高管理（eligible_collaterals, current_collateral_balances）
  - trade_ids参照リスト
  - precalc_exposureフィールド
  - VariationMarginAgreementBuilderによるfluent API
  - _Requirements: 13.1, 13.7, 13.8_

---

- [x] 7. IsdaMasterAgreement構造体の実装
- [x] 7.1 ISDA基本構造の定義
  - IsdaPaymentMethod列挙型（Full, Limited, OnewayToCpty, OnewayToSelf）
  - IsdaInitialMargin構造体（im_post, im_recv, im_currency, im_rate_curve_id）
  - IsdaMasterAgreement構造体の全フィールド
  - variation_margin_agreements, non_csa_trade_ids参照リスト
  - IsdaMasterAgreementBuilderによるfluent API
  - _Requirements: 12.1, 12.2, 12.3, 12.4, 12.6_

- [x] 7.2 (P) ISDA-CSAネッティング分離ロジック
  - CSA付き取引（VMA内）とCSA無し取引（non_csa_trade_ids）の分離管理
  - iter_all_trades()による全トレード横断イテレーション
  - precalc_non_csa_exposureフィールド
  - _Requirements: 12.5, 12.7_

---

- [x] 8. NonNettableTrades（ネッティング不可取引）の実装
- [x] 8.1 (P) NettingEligibilityとNonNettableTrades構造体
  - NettingEligibility列挙型（FullNetting, IsdaOnly, NonNettable）
  - NonNettableTrades構造体（trade_ids, precalc_positive_exposure, precalc_negative_exposure）
  - add_trade()メソッド（重複チェック付き）
  - set_precalc_positive/negative()メソッド
  - _Requirements: 14.1, 14.2, 14.4_

- [x] 8.2 (P) グロスエクスポージャー計算サポート
  - positive/negative exposure分離計算のための構造定義
  - precalc_positive_exposure, precalc_negative_exposureフィールド
  - _Requirements: 14.3, 14.5, 14.6_

---

- [x] 9. CounterpartyPortfolio階層構造の実装
- [x] 9.1 CounterpartyPortfolio構造体の定義
  - counterparty_id, credit_index_id参照
  - isda_agreements: Vec<IsdaMasterAgreement>
  - non_nettable_trades: NonNettableTrades
  - 階層構造の完全表現
  - _Requirements: 15.1, 15.2_

- [x] 9.2 CounterpartyPortfolioBuilderの実装
  - fluent APIによる階層構築（add_isda, add_non_nettable_trade, credit_index）
  - Counterparty一致検証（build()時にCounterpartyMismatchエラー）
  - _Requirements: 15.1_

- [x] 9.3 階層ナビゲーションメソッドの実装
  - iter_all_trades()イテレータ（全階層横断）
  - all_trade_ids() HashSet取得
  - get_all_currencies()通貨集合取得（コールバック方式）
  - get_all_payment_dates()支払日集合取得（コールバック方式）
  - _Requirements: 15.3, 15.4, 15.5_

---

- [x] 10. 事前計算Exposure構造の実装
- [x] 10.1 (P) PreCalculatedExposurePath構造体の実装
  - exposure_by_date: BTreeMap<Date, Vec<f64>>
  - currency: Currency
  - new(), add_exposure(), exposure_at(), dates(), len(), is_empty()メソッド
  - _Requirements: 16.1, 16.4, 16.5_

- [x] 10.2 (P) ExposurePathBuilderの実装
  - 外部システムからの事前計算Exposure構築API
  - 日付/パス整合性バリデーション
  - 通貨バリデーション
  - _Requirements: 16.2, 16.3, 16.6_

---

- [ ] 11. Exposure計算設定構造の実装
- [ ] 11.1 (P) ExposureConfig構造体の実装
  - PfeConfidenceLevel列挙型（Q95, Q97_5, Q99, Custom）
  - ExposureAggregation列挙型（Gross, NetWithinNettingSet, NetWithinCounterparty）
  - ExposureConfig構造体（time_grid, pfe_confidence, aggregation, mpor_config等）
  - デフォルト値の適用
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6_

- [ ] 11.2 (P) CollateralizedExposureConfigの実装
  - 担保付きエクスポージャー計算設定
  - EEPE計算パラメータ（5年ホライゾン、1年有効満期）
  - _Requirements: 6.7, 6.8_

---

- [ ] 12. XVA計算設定構造の実装
- [ ] 12.1 (P) XvaScope/XvaConfigの実装
  - XvaCalculationLevel列挙型（Trade, NettingSet, Counterparty, Book, Portfolio）
  - XvaScope構造体（netting_set_ids, time_horizon, simulation parameters）
  - XvaConfig構造体（CVA/DVA/FVA/KVA/MVA計算フラグ）
  - _Requirements: 5.1, 5.2, 5.3, 5.7, 5.8_

- [ ] 12.2 (P) Funding/Capital/WWR設定の実装
  - FundingConfig構造体（funding_spread_curve_id, collateral_rate_curve_id, funding_currency）
  - CapitalConfig構造体（regulatory_method, capital_rate, risk_weight_multiplier）
  - RegulatoryCapitalMethod列挙型（SaCcr, Imm）
  - WrongWayRiskConfig構造体（correlation_estimate, stress_correlation, model_type）
  - _Requirements: 5.4, 5.5, 5.6_

---

- [ ] 13. Netting計算設定構造の実装
- [ ] 13.1 (P) NettingAgreement構造体の実装
  - NettingAgreementType列挙型（ISDA, GMRA, GMSLA, CSA, Custom）
  - NettingAgreement構造体（legal entity pairs, agreement type, enforceability）
  - NettingJurisdiction構造体（enforceability flags per jurisdiction）
  - _Requirements: 7.1, 7.2, 7.3, 7.5, 7.6_

- [ ] 13.2 (P) CloseoutNetting/PaymentNettingの実装
  - CloseoutNetting構造体（closeout calculation method, timeline）
  - PaymentNetting構造体（operational netting）
  - CrossProductNettingEligibility構造体
  - _Requirements: 7.4, 7.7, 7.8_

---

- [ ] 14. 階層集計機能の実装
- [ ] 14.1 (P) AggregationHierarchy/AggregationConfigの実装
  - AggregationHierarchy列挙型（Trade, NettingSet, Book, Counterparty, Portfolio, LegalEntity）
  - AggregationMethod列挙型（Sum, Average, Max, Min, WeightedAverage）
  - AggregationConfig構造体（grouping keys, aggregation methods）
  - AggregationError列挙型（IncompatibleDimensions）
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.7_

- [ ] 14.2 (P) DrillDownPathの実装
  - 集約データから詳細データへのナビゲーション構造
  - 多次元集計サポート（Book × Currency等）
  - _Requirements: 8.5, 8.6_

---

- [ ] 15. シリアライゼーションの実装
- [ ] 15.1 全構造体へのserde derive追加
  - #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]適用
  - #[serde(rename_all = "camelCase")]でJSON API互換
  - #[serde(skip_serializing_if = "Option::is_none")]でオプションフィールド最適化
  - ID型の文字列表現
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5_

- [ ] 15.2 pricer_riskシリアライゼーション互換性
  - 既存pricer_risk::portfolio形式との互換性確認
  - 必要に応じた変換層追加
  - _Requirements: 10.6_

---

- [ ] 16. 既存コード統合とFrom trait実装
- [ ] 16.1 infra_master → pricer_risk変換の実装
  - infra_master::Bookからpricer_risk互換型へのFrom実装
  - infra_master::CounterpartyPortfolioの変換
  - 型安全な変換とバリデーション
  - _Requirements: 11.1, 11.3_

- [ ] 16.2 モジュール構造とre-exportの整理
  - book/, portfolio/, counterparty/モジュール構成
  - preludeへの公開型追加
  - lib.rsでのモジュール宣言
  - _Requirements: 11.1_

---

- [ ] 17. 単体テストの実装
- [ ] 17.1 (P) Book/Portfolio関連テスト
  - BookBuilder正常系・異常系テスト
  - PortfolioBuilder循環参照検出テスト
  - BookError/PortfolioErrorのDisplay実装テスト
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 9.1, 9.2_

- [ ] 17.2 (P) CounterpartyPortfolio階層テスト
  - IsdaMasterAgreement構築テスト
  - VariationMarginAgreement非対称条件テスト
  - CollateralCallFrequency::default_mpor_days()テスト
  - CounterpartyPortfolio階層イテレーションテスト
  - _Requirements: 12.1, 12.2, 12.3, 13.1, 13.2, 13.3, 13.6, 15.1, 15.3_

- [ ] 17.3 (P) Exposure/XVA設定テスト
  - PreCalculatedExposurePath::validate_time_grid()テスト
  - ExposureConfig/XvaConfigデフォルト値テスト
  - MporDeterminationロジックテスト
  - _Requirements: 6.1, 6.2, 5.1, 5.2, 16.1, 16.4, 17.1_

---

- [ ] 18. 統合テストの実装
- [ ] 18.1 CounterpartyPortfolio構築統合テスト
  - CP → ISDA → VMA → Trade完全階層構築
  - 参照整合性バリデーション検証
  - iter_all_trades()の全トレード取得確認
  - _Requirements: 15.1, 15.2, 15.3, 15.4, 15.5_

- [ ] 18.2 TradeMetadata book_id必須化テスト
  - TradeMetadata構築時のBookIdバリデーション
  - 既存コードパスでのbook_id必須確認
  - TradeBookAssignment履歴管理テスト
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 11.2, 11.5_

- [ ] 18.3 serdeラウンドトリップテスト
  - 全主要構造体のserialize/deserialize往復確認
  - JSON形式の互換性確認
  - _Requirements: 10.1, 10.2, 10.3_

---

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1.1-1.8 | 2.1, 2.2, 17.1 |
| 2.1-2.8 | 3.1, 3.2, 17.1 |
| 3.1-3.7 | 4.1, 4.2, 18.2 |
| 4.1-4.7 | 5.1, 5.2 |
| 5.1-5.8 | 12.1, 12.2, 17.3 |
| 6.1-6.8 | 11.1, 11.2, 17.3 |
| 7.1-7.8 | 13.1, 13.2 |
| 8.1-8.7 | 14.1, 14.2 |
| 9.1-9.7 | 1.2, 1.3, 17.1 |
| 10.1-10.6 | 15.1, 15.2, 18.3 |
| 11.1-11.6 | 4.1, 16.1, 16.2, 18.2 |
| 12.1-12.7 | 1.1, 7.1, 7.2, 17.2 |
| 13.1-13.8 | 6.1, 6.2, 6.3, 17.2 |
| 14.1-14.6 | 8.1, 8.2 |
| 15.1-15.6 | 9.1, 9.2, 9.3, 17.2, 18.1 |
| 16.1-16.6 | 10.1, 10.2, 17.3 |
| 17.1-17.6 | 6.1, 17.3 |
