# Implementation Plan

## Phase 1: 基盤（MVP）

- [x] 1. IR基盤型定義（pricer_core）
- [x] 1.1 (P) AlignedBuffer実装
  - 64バイトアラインメント付きメモリバッファを実装する
  - `std::alloc::Layout`によるカスタムアロケーション
  - `Deref`トレイトでスライスアクセスを提供
  - `Clone`、`Debug`、`Drop`トレイトを実装
  - アラインメント検証用ユニットテストを追加
  - _Requirements: 11.1, 11.2_

- [x] 1.2 (P) PricingKernel構造体定義
  - SoA形式のキャッシュフロー中間表現構造体を定義する
  - 日付配列（payment_dates, fixing_dates）をAlignedBuffer<i32>で保持
  - 計算係数（year_fractions, notionals, spreads, gearings）をAlignedBuffer<f64>で保持
  - インデックス参照（currency_ids, discount_curve_ids, fwd_index_ids, fx_index_ids）をVecで保持
  - メタデータフィールド（len, trade_count）を追加
  - Clone, Debug派生を実装
  - 全配列同一長の不変条件を強制するコンストラクタを実装
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 1.10, 9.1, 9.2_

- [x] 1.3 (P) CompileError型定義
  - コンパイルエラーの構造化表現を定義する
  - UnsupportedInstrument、UnknownIndex、InvalidSchedule等のバリアントを実装
  - thiserrorによるError派生を適用
  - エラーメッセージに詳細コンテキストを含める
  - _Requirements: 2.6, 2.7_

- [x] 2. TradeCompilerフレームワーク（pricer_models）
- [x] 2.1 TradeCompilerトレイト定義
  - Trade階層構造からIRへの変換トレイトを定義する
  - `compile(&Trade) -> Result<CompiledIR, CompileError>`シグネチャ
  - `compile_batch(&[Trade]) -> Result<PricingKernel, CompileError>`シグネチャ
  - CompiledIR列挙型（Linear, Script, Callable）を定義
  - _Requirements: 2.1, 2.8_

- [x] 2.2 IndexMapper実装
  - RateIndex/CurrencyPair→インデックスID変換マッパーを実装する
  - ダミーインデックス規約（fwd_index_ids[0]=0.0, fx_index_ids[0]=1.0）を強制
  - 既存IndexedMarketパターンとの統合
  - _Requirements: 2.3, 4.2, 5.1_

- [x] 3. 線形商品コンパイラ（pricer_models）
- [x] 3.1 LinearProductsCompiler基本実装
  - IRS/Bond/FRAをPricingKernelにコンパイルする基本ロジックを実装する
  - 固定レグ/変動レグのスケジュール展開
  - 支払日の昇順ソート
  - YearFractionの事前計算（DayCountConventionから）
  - TradeCompilerトレイト実装
  - _Requirements: 2.2, 2.4, 2.5, 3.5_

- [x] 3.2 IRS固有コンパイルロジック
  - Interest Rate Swapの固定/変動レグを分離してコンパイルする
  - 固定レグ: gearings=0.0, spreads=fixed_rate
  - 変動レグ: gearings=1.0, spreads=spread, fwd_index_ids=インデックスID
  - アモチ対応（期間ごとの元本変動）
  - _Requirements: 3.1, 3.4_

- [x] 3.3 Bond/FRAコンパイルロジック
  - Bondのクーポンと元本償還をコンパイルする
  - FRAの単一決済キャッシュフローをコンパイルする
  - _Requirements: 3.2, 3.3_

- [x] 3.4 カレンダー・休日調整統合
  - infra_master::Calendarを使用した営業日調整を実装する
  - CalendarCacheによるキャッシュ管理
  - 休日調整適用後の支払日計算
  - _Requirements: 2.4, 9.5_

- [x] 4. CurveProvider・KernelContext（pricer_pricing）
- [x] 4.1 CurveProviderトレイト定義
  - 静的ディスパッチ用の市場データプロバイダートレイトを定義する
  - discount_factor(curve_id, t) -> T
  - forward_rate(index_id, fixing_date) -> T
  - fx_rate(fx_id, t) -> T
  - ダミーインデックス規約をトレイト契約として文書化
  - _Requirements: 8.3, 12.2_

- [x] 4.2 KernelContext実装
  - ジェネリック型パラメータC: CurveProviderによる静的ディスパッチを実装する
  - get_discount_factor, get_forward_rate, get_fx_rateメソッド
  - `#[inline(always)]`によるインライン展開保証
  - Copy/Clone派生
  - _Requirements: 5.3, 8.3, 12.2_

- [x] 4.3 MarketProvider参照実装
  - IndexedMarket<T>からKernelContext用プロバイダーを構築する
  - 割引カーブ/フォワードカーブ/FXレート配列の参照保持
  - CurveProviderトレイト実装
  - _Requirements: 8.3_

- [x] 5. LinearEngine（price_kernel関数）
- [x] 5.1 price_kernel基本実装
  - ブランチレス統一式によるPV計算を実装する
  - 統一式: (L_idx × α + β) × N × τ × FX × DF
  - ループ内に条件分岐なし
  - ジェネリック型パラメータ<T: Float, C: CurveProvider<T>>
  - _Requirements: 8.1, 8.2, 8.4, 12.3_

- [x] 5.2 days_to_yearsヘルパー
  - 評価日からの年単位時間計算を実装する
  - 日付（days from epoch）から年単位時間への変換
  - _Requirements: 9.3, 9.4_

- [x] 6. 統合テスト・検証（Phase 1）
- [x] 6.1 Trade→PricingKernel→PVパイプラインテスト
  - フルコンパイル〜評価パイプラインの統合テストを作成する
  - 単純なIRSでの結果検証
  - 既存price_single_tradeとの結果一致検証
  - _Requirements: 8.5_

- [x] 6.2 (P) AlignedBufferアラインメント検証
  - 64バイトアラインメントの実行時検証テストを作成する
  - ポインタアドレスの下位6ビットが0であることを確認
  - _Requirements: 11.1_

- [x] 6.3 (P) Enzyme AD互換性検証
  - price_kernelのEnzyme微分テストを作成する
  - num-dualとの結果比較
  - スムーズ関数のみ使用確認
  - _Requirements: 12.1, 12.4, 12.5_

## Phase 2: 拡張

- [x] 7. X-Ccy・FX対応
- [x] 7.1 XCcyCompiler実装
  - クロス通貨スワップをPricingKernelにコンパイルする
  - 各レグにFXインデックスIDを割り当て
  - 単一通貨トレードにはダミーFX（fx_index_ids[0]）を割り当て
  - _Requirements: 4.1, 4.3, 4.4_

- [x] 7.2 コラテラル・ファンディング通貨対応
  - 担保通貨と資金調達通貨の分離をサポートする
  - 割引カーブIDの使い分け
  - _Requirements: 4.5_

- [ ] 8. CMS・凸性調整対応
- [ ] 8.1 CMSインデックス統合
  - CMS固有インデックスIDの割り当てを実装する
  - CMSクーポンコンパイル時にCMS用index_idを使用
  - フォワードカーブがCMS凸性調整を透過的に返却
  - _Requirements: 5.1, 5.2, 5.4_

- [x] 9. ScriptKernel（経路依存型）
- [x] 9.1 (P) ScriptKernel構造体定義
  - イベント駆動IR表現構造体を定義する
  - observation_times, ops, constants配列
  - ScriptOp列挙型（CalcFixed, CalcFloat, CheckBarrier, Accumulate, Pay）
  - BarrierType列挙型（UpIn, UpOut, DownIn, DownOut）
  - _Requirements: 6.1, 6.2_

- [x] 9.2 ExoticCompiler（Barrier/Asian）
  - バリアオプション/アジアンオプションをScriptKernelにコンパイルする
  - バリア: CheckBarrierオペレーション生成
  - アジアン: Accumulateオペレーション生成
  - UnsupportedPayoffエラー処理
  - _Requirements: 6.3, 6.4, 6.6_

- [x] 9.3 ScriptEngine実装
  - ScriptKernelを線形シーケンスとして実行する
  - オペレーションコードによるディスパッチ（enumマッチ）
  - 実行時型ディスパッチなし
  - _Requirements: 6.5_

## Phase 3: Callable対応

- [ ] 10. CallableKernel
- [ ] 10.1 (P) CallableKernel構造体定義
  - ブロック構造IR表現構造体を定義する
  - CallableBlock（start_date, end_date, core_flows, exercise）
  - ExerciseDef（exercise_date, exercise_cost, style）
  - ExerciseStyle列挙型（Bermudan, American）
  - _Requirements: 7.1, 7.2_

- [ ] 10.2 CallableCompiler実装
  - Bermudanスワップションを行使日ブロックに分割コンパイルする
  - 原資産スワップを行使日でブロック分割
  - 各ブロックにPricingKernel（core_flows）を生成
  - _Requirements: 7.3_

- [ ] 11. CallableEngine・LSMC
- [ ] 11.1 Forward Pass実装
  - 行使ポイントまでのキャッシュフロー累積を実装する
  - 各ブロックのcore_flowsをprice_kernelで評価
  - 累積PVの記録
  - _Requirements: 7.4_

- [ ] 11.2 LSMCRegressor実装
  - Longstaff-Schwartz最小二乗回帰を実装する
  - nalgebraによるQR分解
  - 継続価値の推定
  - _Requirements: 7.5_

- [ ] 11.3 Backward Pass実装
  - 行使ポイントでの行使/継続判定を実装する
  - 各行使日で行使価値vs継続価値を比較
  - パス更新ロジック
  - _Requirements: 7.5, 7.6_

## Phase 4: 最適化・ベンチマーク

- [ ] 12. パフォーマンス検証
- [ ] 12.1 バッチ評価ベンチマーク
  - 10,000トレードのバッチ評価スループットを測定する
  - criterionベンチマーク
  - 線形スケーリング検証
  - _Requirements: 11.4_

- [ ] 12.2 (P) SIMD検証
  - LLVMベクトル化の確認を行う
  - `perf stat`でSIMD命令カウント
  - キャッシュミス率測定
  - _Requirements: 11.2, 11.3_

- [ ] 12.3 (P) Rayon並列化統合
  - バッチ評価のRayon並列化を実装する
  - トレード単位のpar_iter
  - CPU利用率>80%の検証
  - _Requirements: 11.6_

---

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1.1-1.10 | 1.2 |
| 2.1-2.8 | 1.3, 2.1, 2.2, 3.1 |
| 3.1-3.6 | 3.1, 3.2, 3.3 |
| 4.1-4.5 | 2.2, 7.1, 7.2 |
| 5.1-5.4 | 2.2, 4.2, 8.1 |
| 6.1-6.6 | 9.1, 9.2, 9.3 |
| 7.1-7.6 | 10.1, 10.2, 11.1, 11.2, 11.3 |
| 8.1-8.6 | 4.1, 4.2, 4.3, 5.1, 6.1 |
| 9.1-9.5 | 1.2, 3.4, 5.2 |
| 10.1-10.7 | All tasks (A-I-P-S compliance verified through layer placement) |
| 11.1-11.6 | 1.1, 6.2, 12.1, 12.2, 12.3 |
| 12.1-12.6 | 4.1, 4.2, 5.1, 6.3 |
