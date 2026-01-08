# Implementation Plan

## Task 1: クレート名変更とワークスペ�Eス再構�E

- [x] 1.1 pricer_kernelからpricer_pricingへのクレート名変更
  - Cargo.tomlのパッケージ名をpricer_pricingに更新
  - crates/チE��レクトリをpricer_kernelからpricer_pricingにリネ�Eム
  - ワークスペ�EスCargo.tomlのmembersを更新
  - 旧名でのエイリアスを提供！Eeprecation警告付き�E�E
  - 全クレート�E依存関係参照を更新
  - _Requirements: 7.1_

- [x] 1.2 (P) pricer_xvaからpricer_riskへのクレート名変更
  - Cargo.tomlのパッケージ名をpricer_riskに更新
  - crates/チE��レクトリをpricer_xvaからpricer_riskにリネ�Eム
  - ワークスペ�EスCargo.tomlのmembersを更新
  - 旧名でのエイリアスを提供！Eeprecation警告付き�E�E
  - 全クレート�E依存関係参照を更新
  - _Requirements: 7.1_

- [x] 1.3 依存関係グラフ�E検証とビルド確誁E
  - cargo buildで全クレート�Eビルド�E功を確誁E
  - cargo testで既存テスト�Eパス確誁E
  - L1→L2→L3→L4の依存方向�Eみであることを検証
  - 循環依存が発生してぁE��ぁE��とを確誁E
  - _Requirements: 7.5_

## Task 2: Feature Flag設定とアセチE��クラス別条件付きコンパイル

- [x] 2.1 pricer_modelsへのfeature flag追加
  - Cargo.tomlにequity, rates, credit, fx, commodity, exoticフィーチャーを定義
  - default = ["equity"]を設宁E
  - all = ["equity", "rates", "credit", "fx", "commodity", "exotic"]を設宁E
  - 吁E��ィーチャー間�E依存関係を適刁E��設宁E
  - _Requirements: 7.4_

- [x] 2.2 instruments/配下�EアセチE��クラス別サブモジュール作�E
  - equity/サブモジュールを作�Eし、既存�EVanillaOption, Barrier, Asian, Lookbackを移勁E
  - rates/サブモジュールを作�E�E�スケルトン�E�E
  - credit/サブモジュールを作�E�E�スケルトン�E�E
  - fx/サブモジュールを作�E�E�スケルトン�E�E
  - commodity/サブモジュールを作�E�E�スケルトン�E�E
  - exotic/サブモジュールを作�E�E�スケルトン�E�E
  - 吁E��ブモジュールを対応するfeature flagでゲーチE
  - _Requirements: 7.2, 1.1_

- [x] 2.3 models/配下�EモチE��カチE��リ別サブモジュール作�E
  - equity/サブモジュールを作�Eし、既存�EGBMを移勁E
  - rates/サブモジュールを作�E�E�スケルトン�E�E
  - hybrid/サブモジュールを作�E�E�スケルトン�E�E
  - 既存�EStochasticModel traitとStochasticModelEnumを維持E
  - _Requirements: 7.3_

## Task 3: マルチカーブ市場チE�Eタ基盤

- [x] 3.1 CurveNameとCurveEnumの定義
  - カーブ名の列挙型を定義�E�EIS, SOFR, TONAR, Euribor, Forward, Discount, Custom�E�E
  - 既存�EFlatCurve, InterpolatedCurveを統合するenum型を定義
  - 全型をジェネリチE��Float型で維持しAD互換性を確俁E
  - _Requirements: 2.5_

- [x] 3.2 CurveSet構造体�E実裁E
  - 褁E��カーブを名前付きで管琁E��るコンチE��を実裁E
  - カーブ�E登録・取得メソチE��を実裁E
  - チE��スカウントカーブとフォワードカーブ�E刁E��取得を実裁E
  - _Requirements: 2.1, 2.2_

- [x] 3.3 CreditCurveトレイトとHazardRateCurve実裁E
  - ハザードレート�E生存確玁E�EチE��ォルト確玁E�E計算インターフェースを定義
  - チE��ーとハザードレート�E期間構造を持つ実裁E��作�E
  - 生存確玁E�E積�E計算を実裁E
  - _Requirements: 2.3, 5.3_

- [x] 3.4 MarketDataError拡張
  - カーブ未検�Eエラーを追加
  - 無効な満期エラーを追加
  - 補間失敗エラーを追加
  - 欠損データの詳細を含むエラーメチE��ージを提侁E
  - _Requirements: 2.4_

- [x] 3.5 (P) FxVolatilitySurface実裁E
  - チE��タ×満期グリチE��でのボラチE��リチE��サーフェスを実裁E
  - VolatilitySurfaceトレイトを実裁E
  - ATMボラチE��リチE��取得メソチE��を追加
  - _Requirements: 6.5_

## Task 4: Instrumentトレイトと階層的Enum再構�E

- [x] 4.1 Instrumentトレイト�E定義
  - 価格計算�EGreeks計算�EキャチE��ュフロー取得メソチE��を定義
  - 満期�E通貨取得メソチE��を追加
  - 全啁E��が実裁E��べき�E通インターフェースを確竁E
  - _Requirements: 1.3_

- [x] 4.2 InstrumentEnumの階層皁Enum構造への再構�E
  - トップレベルのアセチE��クラス別enumを定義�E�Equity, Rates, Credit, Fx, Commodity, Exotic�E�E
  - 株式商品�Eサブenumを定義
  - 既存�E啁E��をEquityサブenumに統吁E
  - トレイト実裁E��enumに委譲
  - 静的チE��スパッチを維持E
  - _Requirements: 1.2, 1.5_

- [x] 4.3 後方互換性の維持E
  - 既存�EInstrument enum APIを維持E
  - 新旧両方のアクセスパターンをサポ�EチE
  - deprecation警告で移行を俁E��
  - _Requirements: 1.5_

## Task 5: スケジュール生成モジュール

- [x] 5.1 基本型の定義
  - スケジュール構造体を定義（期間リスト、支払日、計算期間開始・終了）
  - 期間構造体を定義（開始、終了、支払日、日数計算規約）
  - 頻度列挙型を定義（年次、半期、四半期、月次、週次、日次）
  - _Requirements: 1.4_

- [x] 5.2 BusinessDayConventionとDayCountConvention拡張
  - 営業日調整規約の列挙型を定義（Following, ModifiedFollowing, Preceding, ModifiedPreceding, Unadjusted）
  - 既存の日数計算規約（Act360, Act365, Thirty360）を確認
  - _Requirements: 4.5_

- [x] 5.3 ScheduleBuilderの実装
  - ビルダーパターンでスケジュール生成器を実装
  - 開始日、終了日、頻度、日数計算規約の設定メソッドを実装
  - 基本的なスケジュール生成をサポート
  - _Requirements: 4.5_

## Task 6: 金利デリバティブ商品

- [x] 6.1 (P) InterestRateSwap構造体の実装
  - IRS構造体を定義（ノーショナル、固定レグ、変動レグ、通貨）
  - 固定レグ構造体を定義（スケジュール、固定レート、日数計算規約）
  - 変動レグ構造体を定義（スケジュール、スプレッド、インデックス、日数計算規約）
  - レートインデックス列挙型を定義（SOFR, TONAR, Euribor3M, Euribor6M）
  - 金利商品サブenumにSwapバリアントを追加
  - _Requirements: 4.1_

- [x] 6.2 (P) Swaption構造体の実装
  - Swaption構造体を定義（原資産スワップ、満期、ストライク、オプションタイプ）
  - Swaptionタイプ列挙型を定義（Payer, Receiver）
  - 金利商品サブenumにSwaptionバリアントを追加
  - _Requirements: 4.3_

- [x] 6.3 (P) Cap/Floor構造体の実装
  - Cap構造体を定義（ノーショナル、スケジュール、ストライク、インデックス）
  - Floor構造体を定義（同様）
  - Collar構造体を定義（キャップストライク、フロアストライク）
  - 金利商品サブenumにCap, Floorバリアントを追加
  - _Requirements: 4.3_

- [x] 6.4 IRS評価ロジックの実装
  - フォワードレート計算ロジックを実装
  - 変動レグと固定レグのキャッシュフロー計算を実装
  - CurveSetからディスカウント・フォワードカーブを取得して評価
  - _Requirements: 4.2_

- [x] 6.5 Swaption解析解の実装
  - Black76モデルによるSwaption価格計算を実装
  - Bachelierモデル（正規モデル）によるSwaption価格計算を実装
  - 解析解モジュールに追加
  - _Requirements: 4.4_

## Task 7: クレジチE��チE��バティブ商品E

- [x] 7.1 (P) CreditDefaultSwap構造体�E実裁E
  - CDS構造体を定義�E�参照エンチE��チE��、ノーショナル、スプレチE��、満期！E
  - クレジチE��啁E��サブenumにCdsバリアントを追加
  - Instrumentトレイトを実裁E
  - _Requirements: 5.1_

- [x] 7.2 CDS評価ロジチE��の実裁E
  - ハザードレートカーブから生存確玁E��計箁E
  - プロチE��ションレグPV計算を実裁E
  - プレミアムレグPV計算を実裁E
  - _Requirements: 5.2_

- [x] 7.3 チE��ォルトシミュレーションのMC拡張
  - 生存確玁E�E送E��数でチE��ォルト時刻をサンプリング
  - MCエンジンにチE��ォルトイベント�E琁E��追加
  - _Requirements: 5.4_

## Task 8: 為替デリバティブ商品

- [x] 8.1 CurrencyPair構造体の実装
  - 通貨ペア構造体を定義（ベース通貨、クォート通貨）
  - スポットレート管理機能を追加
  - 既存のCurrency列挙型を活用
  - _Requirements: 6.3_

- [x] 8.2 (P) FxOption構造体の実装
  - FXオプション構造体を定義（通貨ペア、ストライク、満期、オプションタイプ）
  - 為替資産サブenumにOptionバリアントを追加
  - Instrumentトレイトを実装
  - _Requirements: 6.1_

- [x] 8.3 (P) FxForward構造体の実装
  - FXフォワード構造体を定義（通貨ペア、フォワードレート、満期、ノーショナル）
  - 為替資産サブenumにForwardバリアントを追加
  - _Requirements: 6.1_

- [x] 8.4 Garman-Kohlhagenモデルの実装
  - GK公式によるFXオプション価格計算を実装
  - 国内・外国金利カーブを使用したディスカウント
  - 解析解モジュールに追加
  - _Requirements: 6.2_

## Task 9: 確玁E��チE��拡張

- [ ] 9.1 StochasticModelトレイトへのnum_factors追加
  - ファクター数取得メソチE��を追加
  - 既存�EGBMでnum_factors = 1を実裁E
  - 1ファクター/2ファクター/マルチファクターモチE��の統一皁E��ぁE
  - _Requirements: 3.1_

- [ ] 9.2 Hull-White 1FモチE��の実裁E
  - Hull-White構造体を定義�E�平坁E��帰速度、�EラチE��リチE��、�E期カーブ！E
  - StochasticModelトレイトを実裁E
  - 短期��利パス生�Eのevolve_stepを実裁E
  - 金利モチE��サブモジュールに追加
  - _Requirements: 3.2_

- [ ] 9.3 (P) CIRモチE��の実裁E
  - Cox-Ingersoll-Ross構造体を定義
  - StochasticModelトレイトを実裁E
  - Feller条件のバリチE�Eションを追加
  - 金利モチE��サブモジュールに追加
  - _Requirements: 3.2_

- [ ] 9.4 StochasticModelEnumへの新モチE��追加
  - HullWhite, CIRバリアントをStochasticModelEnumに追加
  - 静的チE��スパッチを維持E
  - enumバリアント追加のみで拡張可能な構造を確誁E
  - _Requirements: 3.3_

- [ ] 9.5 CorrelatedModels�E�Eholesky刁E���E��E実裁E
  - 相関モチE��構造体を定義�E�モチE��リスト、相関行�E、Cholesky刁E��済み行�E�E�E
  - 相関行�EのCholesky刁E��を実裁E
  - 相関ブラウン運動の生�Eを実裁E
  - ハイブリチE��モチE��サブモジュールに追加
  - _Requirements: 3.4_

## Task 10: キャリブレーション基盤

- [x] 10.1 Levenberg-Marquardtソルバ�Eの実裁E
  - LMソルバ�E構造体を定義
  - 非線形最小二乗法�E反復計算を実裁E
  - 収束判定とパラメータ更新ロジチE��を実裁E
  - 数学ソルバ�Eモジュールに追加
  - _Requirements: 8.3_

- [x] 10.2 Calibratorトレイト�E定義
  - キャリブレーション・目皁E��数・制紁E��件メソチE��を定義
  - キャリブレーション結果構造体を定義�E�収束フラグ、イチE��ーション数、残差、最終パラメータ�E�E
  - 制紁E��件構造体を定義
  - _Requirements: 8.1_

- [x] 10.3 CalibrationError型�E実裁E
  - キャリブレーションエラー構造体を定義�E�種別、残差、イチE��ーション数、メチE��ージ�E�E
  - エラー種別列挙型を定義�E�EotConverged, InvalidConstraint, NumericalInstability, InsufficientData�E�E
  - 残差、イチE��ーション数、収束判定基準を含む詳細惁E��を提侁E
  - _Requirements: 8.4_

- [x] 10.4 SwaptionCalibratorの実裁E
  - Swaptionキャリブレータ構造体を定義
  - SwaptionボラチE��リチE��サーフェスへのキャリブレーションを実裁E
  - Hull-WhiteモチE��のパラメータ�E�平坁E��帰速度、�EラチE��リチE���E�を推宁E
  - _Requirements: 8.2_

- [x] 10.5 Enzyme ADを活用した勾配計箁E
  - キャリブレーション目皁E��数のAD対忁E
  - 勾配計算によるキャリブレーション高速化
  - enzyme-modeでのチE��チE
  - _Requirements: 8.5_

## Task 11: リスクファクター管琁E

- [ ] 11.1 RiskFactorトレイト�E定義
  - リスクファクタートレイトを定義�E�ファクター種別、バンプ、シナリオ適用�E�E
  - リスクファクター種別列挙型を定義�E�EnterestRate, Credit, Fx, Equity, Commodity, Volatility�E�E
  - トレイトモジュールに追加
  - _Requirements: 9.1_

- [ ] 11.2 BumpScenarioとRiskFactorShiftの実裁E
  - リスクファクターシフト構造体を定義�E�ファクター種別、シフト種別、値�E�E
  - シフト種別列挙型を定義�E�Ebsolute, Relative, Parallel, Twist, Butterfly�E�E
  - 吁E��スクファクターの独立�E同時シフトをサポ�EチE
  - _Requirements: 9.2_

- [ ] 11.3 GreeksAggregatorの実裁E
  - Greeks雁E��構造体を定義
  - 雁E��手法�E挙型を定義�E�Eimple, RiskWeighted, CorrelationAdjusted�E�E
  - ポ�EトフォリオレベルDelta, Gamma, Vegaを計算するメソチE��を実裁E
  - ポ�EトフォリオGreeks構造体を定義
  - _Requirements: 9.3_

- [ ] 11.4 ScenarioEngineの実裁E
  - シナリオエンジン構造体を定義
  - シナリオ構造体を定義�E�名前、シフトリスト！E
  - シナリオ実行�E全シナリオ実行メソチE��を実裁E
  - シナリオPnL構造体を定義
  - _Requirements: 9.4_

- [ ] 11.5 プリセチE��シナリオの追加
  - パラレルシフト�E�E1bp, +10bp, +100bp�E�を追加
  - チE��ストシナリオ�E�短期�E長期�E�E�を追加
  - バタフライシナリオ�E�中期スパイク�E�を追加
  - プリセチE��モジュールに追加
  - _Requirements: 9.5_

## Task 12: エキゾチックチE��バティブ商品E

- [ ] 12.1 (P) VarianceSwap構造体�E実裁E
  - バリアンススワチE�E構造体を定義�E�ストライク、バリアンスノ�Eショナル、観測日�E�E
  - ボラチE��リチE��スワチE�E構造体を定義�E�バリアンススワチE�Eとの違いを�E確化！E
  - エキゾチック啁E��サブenumにVarianceSwap, VolatilitySwapバリアントを追加
  - _Requirements: 11.1, 11.8_

- [ ] 12.2 VarianceSwap評価ロジチE��の実裁E
  - ログリターンの二乗和から実現バリアンスを計箁E
  - レプリケーションまた�EMCで公正バリアンスストライクを算�E
  - _Requirements: 11.2_

- [ ] 12.3 (P) Cliquet構造体�E実裁E
  - クリケチE��構造体を定義�E�リセチE��日、ローカルキャチE�E/フロア、グローバルキャチE�E/フロア�E�E
  - エキゾチック啁E��サブenumにCliquetバリアントを追加
  - _Requirements: 11.3_

- [ ] 12.4 (P) Autocallable構造体�E実裁E
  - オートコーラブル構造体を定義�E�観測日、早期償邁E��リア、クーポン条件、ノチE��インプット！E
  - エキゾチック啁E��サブenumにAutocallableバリアントを追加
  - _Requirements: 11.4_

- [ ] 12.5 (P) Rainbow�E�EestOf/WorstOf�E�構造体�E実裁E
  - レインボ�E構造体を定義�E�原賁E��リスト、�Eイオフタイプ、相関�E�E
  - ペイオフタイプ�E挙型を定義�E�EestOf, WorstOf, Spread�E�E
  - エキゾチック啁E��サブenumにRainbowバリアントを追加
  - _Requirements: 11.5_

- [ ] 12.6 (P) QuantoOption構造体�E実裁E
  - クオントオプション構造体を定義�E�原賁E��通貨、決済通貨、クオント調整�E�E
  - エキゾチック啁E��サブenumにQuantoバリアントを追加
  - _Requirements: 11.6_

- [ ] 12.7 Longstaff-Schwartz法�E実裁E
  - LSM構造体を定義�E�基底関数タイプ、基底関数数、two-pass使用フラグ�E�E
  - 基底関数タイプ�E挙型を定義�E�Eolynomial, Laguerre, Hermite�E�E
  - 継続価値計算メソチE��を実裁E
  - 行使墁E��探索メソチE��を実裁E
  - アメリカンオプションモジュールに追加
  - _Requirements: 11.7_

- [ ] 12.8 BermudanSwaption評価の実裁E
  - バミューダンSwaption構造体を定義
  - Longstaff-Schwartz法による早期行使墁E��推宁E
  - エキゾチック啁E��サブenumにBermudanバリアントを追加
  - _Requirements: 11.7_

## Task 13: XVA拡張

- [ ] 13.1 Wrong-Way Riskモジュールの実裁E
  - Wrong-Way Risk構造体を定義
  - CVA計算でのWWR老E�Eオプションを追加
  - 相関パラメータによるエクスポ�Eジャー調整
  - XVAモジュールに追加
  - _Requirements: 5.5_

- [ ] 13.2 (P) マルチカレンシーXVA対忁E
  - 吁E��引�E決済通貨と評価通貨の変換を�E動�E琁E
  - CurrencyPairを使用した為替レート適用
  - _Requirements: 6.4_

## Task 14: パフォーマンス最適匁E

- [ ] 14.1 SoAレイアウト�E維持確誁E
  - pricer_riskでのSoA構造の維持を確誁E
  - 新規構造体でもSoA対応を検訁E
  - _Requirements: 10.1_

- [ ] 14.2 (P) Rayon並列化の適用確誁E
  - Portfolio評価の並列化が維持されてぁE��ことを確誁E
  - 新規アセチE��クラスでの並列評価チE��チE
  - _Requirements: 10.2_

- [ ] 14.3 (P) ワークスペ�Eスバッファパターンの適用
  - 新規評価パスでのバッファ再利用を確誁E
  - メモリアロケーション最小化の検証
  - _Requirements: 10.3_

- [ ] 14.4 (P) チェチE��ポイント機�Eの統合確誁E
  - 新規モチE��・啁E��でのcheckpointing対忁E
  - メモリ使用量と再計算�Eトレードオフ設宁E
  - _Requirements: 10.4_

- [ ] 14.5 criterionベンチ�Eーク追加
  - IRS評価ベンチ�Eーク追加
  - CDS評価ベンチ�Eーク追加
  - FXオプション評価ベンチ�Eーク追加
  - Swaption評価ベンチ�Eーク追加
  - パフォーマンス回帰検�Eの仕絁E��確誁E
  - _Requirements: 10.5_

## Task 15: 統合テスチE

- [ ] 15.1 IRS評価の統合テスチE
  - Schedule生�E ↁECurveSet構篁EↁEprice()呼び出ぁEↁE既知値との比輁E
  - HullWhiteモチE��でのMC評価チE��チE
  - _Requirements: 4.1, 4.2_

- [ ] 15.2 (P) Swaption評価の統合テスチE
  - HullWhiteキャリブレーション ↁEMC価格 ↁEBlack76解析解との比輁E
  - キャリブレーション収束確誁E
  - _Requirements: 4.3, 4.4, 8.2_

- [ ] 15.3 (P) CDS評価の統合テスチE
  - HazardRateCurve構篁EↁEプロチE��ション/プレミアムレグPV計箁E
  - 既知のCDSスプレチE��との整合性確誁E
  - _Requirements: 5.1, 5.2_

- [ ] 15.4 (P) FXオプション評価の統合テスチE
  - Garman-KohlhagenモチE��での価格計箁E
  - マルチカレンシーポ�EトフォリオでのXVA計箁E
  - _Requirements: 6.1, 6.2, 6.4_

- [ ] 15.5 全アセチE��クラス横断のPortfolio XVAチE��チE
  - Equity + Rates + Credit + FX啁E��の混合�Eートフォリオ
  - ExposureProfile生�E ↁECVA/DVA/FVA計箁E
  - 吁E��スクファクターの感応度確誁E
  - _Requirements: 9.3, 9.4, 5.5_

- [ ] 15.6 (P) エキゾチック啁E��の統合テスチE
  - VarianceSwap評価とGreeks計箁E
  - Autocallable/Cliquetの早期償邁E��ミュレーション
  - BermudanSwaptionのLSM評価
  - _Requirements: 11.1, 11.2, 11.7_
