# Implementation Plan

## Phase 1: 既存機能完成とコア基盤強化

- [ ] 1. SABRモンテカルロパス生成の完成
- [ ] 1.1 SABRモデルのMCパス生成ロジック実装
  - SABRパラメータ(alpha, beta, rho, nu)を用いた確率過程のevolve_step実装
  - Hagan近似とMC生成の整合性検証
  - 既存SABRキャリブレーション結果との統合
  - _Requirements: 9.3_

- [ ] 1.2 SABRパス生成のユニットテストと検証
  - 解析解(Hagan公式)との比較テスト
  - パラメータ境界条件でのロバスト性確認
  - キャリブレーション収束率95%以上の達成確認
  - _Requirements: 9.3, 18.4_

- [ ] 2. コモディティ商品モジュールの完成
- [ ] 2.1 (P) Schwartz one-factor平均回帰モデル実装
  - 平均回帰速度(kappa)、長期平均(theta)、ボラティリティのパラメータ構造体
  - StochasticModelトレイト実装(evolve_step、initial_state)
  - コンビニエンスイールドの組み込み
  - _Requirements: 4.3_

- [ ] 2.2 (P) 季節性フォワードカーブ構築
  - 季節性パターン(月次/四半期)のモデリング
  - 補間メソッド(線形、スプライン)との統合
  - エネルギー・農産物向けカーブ生成
  - _Requirements: 4.4_

- [ ] 2.3 コモディティ商品構造体の実装
  - コモディティフォワード/先物のペイオフ計算
  - コモディティオプション(ヨーロピアン、アジアン)対応
  - スプレッドオプション(カレンダー、クラック)実装
  - _Requirements: 4.1, 4.2, 4.5_

- [ ] 2.4 スウィングオプション(複数行使権)実装
  - 行使権の管理と状態遷移
  - 最適行使境界の近似計算
  - MCシミュレーションでの行使判定
  - _Requirements: 4.6_

- [ ] 3. クレジット商品モジュールの拡張
- [ ] 3.1 (P) CDS指数(CDX/iTraxx)プライシング
  - 指数ファクターと残存銘柄数の管理
  - 構成銘柄のハザードレート集約
  - 指数スプレッドからの逆算
  - _Requirements: 5.2_

- [ ] 3.2 (P) CDSスワプションプライシング
  - CDSスワプションのペイオフ構造
  - フォワードCDSスプレッドの計算
  - Black/Bachelierモデルでの価格計算
  - _Requirements: 5.3_

- [ ] 3.3 (P) 合成CDOトランシェプライシング
  - アタッチメント/デタッチメントポイントの設定
  - ベースコリレーション曲面の構築
  - 期待損失計算によるトランシェ価値
  - _Requirements: 5.4_

- [ ] 3.4 Contingent CDS(CCDS)とWrong-Way Riskモデリング
  - エクスポージャーとデフォルト確率の相関モデル
  - CCDS固有のペイオフ構造
  - WWRパラメータのキャリブレーション
  - _Requirements: 5.6_

- [ ] 3.5 (P) Credit-Linked Note(CLN)プライシング
  - 元本リンク型ペイオフ構造
  - 発行体信用リスクの組み込み
  - 債券価値とCDSプロテクションの統合
  - _Requirements: 5.7_

- [ ] 3.6 ハザードレートキャリブレーション統合
  - CDSスプレッドからのハザードレート逆算
  - ブートストラップ法による信用カーブ構築
  - 既存クレジットカーブインフラとの統合
  - _Requirements: 5.5_

## Phase 2: PDE法と解析解拡張

- [ ] 4. PDEソルバーエンジンの実装
- [ ] 4.1 有限差分スキーム(陽的/陰的/Crank-Nicolson)実装
  - 三重対角行列ソルバーの実装
  - 陽的・陰的・Crank-Nicolsonスキームの選択機構
  - 安定性条件(CFL数)の検証
  - _Requirements: 8.1_

- [ ] 4.2 適応的グリッド構築機能
  - ストライク/バリア近傍でのグリッド細分化
  - sinh変換によるグリッド集中
  - カスタム非一様グリッドサポート
  - _Requirements: 8.3_

- [ ] 4.3 境界条件処理(Dirichlet/Neumann/吸収境界)
  - 各境界条件タイプの実装
  - バリアオプション用吸収境界
  - 境界値の時間依存性対応
  - _Requirements: 8.4_

- [ ] 4.4 アメリカンオプション用PSOR法実装
  - Projected Successive Over-Relaxation反復法
  - 早期行使境界の計算
  - 収束判定と最大反復回数制御
  - _Requirements: 8.2_

- [ ] 4.5 (P) ADI法(多因子モデル対応)実装
  - Alternating Direction Implicit法
  - 2因子モデル(Heston等)への適用
  - 分割誤差の制御
  - _Requirements: 8.5_

- [ ] 4.6 グリッド収束解析とRichardson外挿
  - 異なるグリッド解像度での結果比較
  - Richardson外挿による精度向上
  - 収束オーダーの自動判定
  - _Requirements: 8.6_

- [ ] 5. Local Volatilityモデル実装
- [ ] 5.1 Dupire公式によるLocal Vol曲面抽出
  - インプライドボラティリティ曲面からのLocal Vol計算
  - 数値微分(strike/time)の安定化
  - 無裁定条件の検証
  - _Requirements: 9.6_

- [ ] 5.2 Local VolモデルのMCパス生成
  - Local Vol曲面を用いたevolve_step実装
  - 曲面補間(bilinear、cubic)の選択
  - AD互換性の確保(スムージング関数使用)
  - _Requirements: 9.6, 10.5_

- [ ] 5.3 Local Volキャリブレーション
  - バニラオプション価格へのフィッティング
  - 正則化(Tikhonov)による安定化
  - キャリブレーション品質メトリクス出力
  - _Requirements: 12.4_

- [ ] 6. Jump-Diffusionモデル実装
- [ ] 6.1 Merton Jump-Diffusionモデル実装
  - ポアソンジャンプ過程の生成
  - 対数正規ジャンプサイズ分布
  - GBM拡散成分との統合
  - _Requirements: 9.7_

- [ ] 6.2 Kou Double-Exponentialモデル実装
  - 二重指数分布ジャンプサイズ
  - 上昇/下降確率パラメータ
  - 解析解(ヨーロピアン)との検証
  - _Requirements: 9.7_

- [ ] 6.3 Jump-DiffusionのAD互換性対応
  - ジャンプ発生のスムージング処理
  - ポアソン過程の連続近似
  - Enzyme ADでの動作検証(注意: 不連続性処理が必要)
  - _Requirements: 9.7, 10.5_

- [ ] 7. 解析解の拡張
- [ ] 7.1 (P) 金利商品解析解(Black/Bachelier for Swaptions)
  - スワプション用Black公式実装
  - Bachelierモデル(正規ボラティリティ)実装
  - Cap/Floor解析解
  - _Requirements: 6.5_

- [ ] 7.2 (P) バリアオプション解析解の拡張
  - 全8種類(up/down, in/out, call/put)の完全実装
  - リベート付きバリアオプション
  - 連続監視vs離散監視の補正
  - _Requirements: 6.4_

- [ ] 7.3 解析的Greeks計算の完成
  - 全解析解モデルに対するDelta/Gamma/Vega/Theta/Rho
  - 二次Greeks(Vanna, Volga, Charm, Vomma)
  - 解析解Greeks vs 有限差分の検証
  - _Requirements: 6.6_

## Phase 3: アセットクラス拡充

- [ ] 8. 金利商品プライシング完成
- [ ] 8.1 IRS(金利スワップ)キャッシュフロー生成
  - 固定/変動レッグのキャッシュフロー計算
  - デイカウントコンベンション対応
  - マルチカーブ(割引/プロジェクション分離)
  - _Requirements: 1.1, 1.7_

- [ ] 8.2 スワプションプライシング(Black/Bachelier/SABR)
  - スワプションペイオフ構造
  - 各ボラティリティモデルでの価格計算
  - SABRスマイルキャリブレーション統合
  - _Requirements: 1.2, 9.3_

- [ ] 8.3 (P) Cap/Floorプライシング
  - Caplet/Floorlet分解
  - Black/Bachelierモデル適用
  - ストリップ価格からのブートストラップ
  - _Requirements: 1.3_

- [ ] 8.4 (P) クロスカレンシースワップ(XCCY)
  - FXエクスポージャー計算
  - ベーシススプレッド対応
  - 担保通貨の影響反映
  - _Requirements: 1.4_

- [ ] 8.5 (P) OIS(Overnight Indexed Swap)プライシング
  - 日次複利計算
  - OISカーブ構築との統合
  - ロックアウト期間対応
  - _Requirements: 1.5_

- [ ] 8.6 (P) ベーシススワップ(異なる指数間)
  - 2つの変動レッグ(異なるインデックス)
  - ベーシススプレッドの取り扱い
  - テナーベーシス対応
  - _Requirements: 1.6_

- [ ] 8.7 (P) インフレーションスワップ
  - ゼロクーポンインフレスワップ
  - Year-on-Yearインフレスワップ
  - インフレ指数カーブ構築
  - _Requirements: 1.8_

- [ ] 9. FX商品プライシング完成
- [ ] 9.1 FXバニラオプション(Garman-Kohlhagen)拡張
  - 完全なGarman-Kohlhagen実装
  - Smile対応(SABR/Mixed Log-Normal)
  - _Requirements: 2.1, 2.7_

- [ ] 9.2 (P) FXフォワード・NDF
  - Non-Deliverable Forward構造
  - フィキシングレートと決済
  - NDFペイオフ計算
  - _Requirements: 2.2_

- [ ] 9.3 FXバリアオプション(全8種類)
  - 全種類(up/down, in/out, call/put)対応
  - スムーズバリア検出(AD互換)
  - 連続/離散監視の切り替え
  - _Requirements: 2.3_

- [ ] 9.4 (P) FXデジタルオプション
  - スムーズペイオフ近似(シグモイド)
  - AD互換性の確保
  - One-Touch/No-Touch対応
  - _Requirements: 2.4, 2.5_

- [ ] 9.5 (P) FXアジアンオプション
  - 算術/幾何平均
  - ストリーミング統計でのMC計算
  - Kemna-Vorstとの検証
  - _Requirements: 2.6_

- [ ] 9.6 マルチ通貨決済とFXレート三角裁定
  - 複数通貨間のレート整合性
  - 三角裁定フリー条件の検証
  - クロスレート計算
  - _Requirements: 2.8_

- [ ] 10. 株式商品プライシング拡張
- [ ] 10.1 アメリカンオプション(PDE法適用)
  - PDEソルバーとの統合
  - 早期行使境界の可視化
  - 配当モデリング対応
  - _Requirements: 3.1, 8.2_

- [ ] 10.2 (P) ルックバックオプション拡張
  - 固定/変動ストライク対応
  - ストリーミングmin/max追跡
  - 解析解との検証
  - _Requirements: 3.4_

- [ ] 10.3 (P) オートコーラブル商品
  - 早期償還条件の判定
  - クーポンとノックアウト条件
  - パス依存MCでの評価
  - _Requirements: 3.5_

- [ ] 10.4 (P) バスケットオプション(Worst-of/Best-of)
  - 複数原資産の相関モデリング
  - Cholesky分解による相関パス生成
  - Worst-of/Best-of ペイオフ
  - _Requirements: 3.6, 9.8_

- [ ] 10.5 配当モデリング(離散配当/連続利回り)
  - 離散配当のジャンプ処理
  - 連続配当利回り
  - 配当スケジュール管理
  - _Requirements: 3.8_

- [ ] 10.6 Hestonモデルフルキャリブレーション
  - バニラオプション曲面へのフィッティング
  - Levenberg-Marquardt最適化
  - パラメータ制約(Feller条件等)
  - _Requirements: 9.2, 12.2_

## Phase 4: リスク・規制計算

- [ ] 11. XVA計算拡張
- [ ] 11.1 ColVA(Collateral Valuation Adjustment)
  - CSA条件のモデリング
  - 担保金利と無リスク金利の差
  - 担保プロファイル計算
  - _Requirements: 15.4_

- [ ] 11.2 (P) KVA(Capital Valuation Adjustment)
  - ハードルレート(自己資本コスト)
  - 資本計算手法(SA-CCR/IMM)との連携
  - 資本プロファイルの時間積分
  - _Requirements: 15.5_

- [ ] 11.3 (P) MVA(Margin Valuation Adjustment)
  - 初期証拠金モデル(SIMM等)
  - IM調達コスト
  - IMプロファイル計算
  - _Requirements: 15.6_

- [ ] 11.4 ネッティングセット集約とCSA条件対応
  - 複数取引のネッティング
  - CSA閾値・MTA対応
  - 担保ポスティングロジック
  - _Requirements: 15.7_

- [ ] 11.5 インクリメンタルXVA計算
  - 取引レベル帰属計算
  - 差分XVA算出
  - ポートフォリオへの追加/削除影響
  - _Requirements: 15.8_

- [ ] 11.6 規制タイムポイントでのエクスポージャープロファイル
  - 規制所定タイムグリッド(IMM日付等)
  - EE, EPE, PFE, EEPE, ENEの規制計算
  - エクスポージャー補間/外挿
  - _Requirements: 15.9_

- [ ] 12. SA-CCRエンジン実装
- [ ] 12.1 アセットクラス別アドオン計算
  - 金利/FX/クレジット/株式/コモディティ分類
  - 監督因子の適用
  - アドオン集計ロジック
  - _Requirements: 16.1_

- [ ] 12.2 Replacement CostとPFE計算
  - 置換コスト計算(V - C)
  - ヘッジセット集約
  - マルチプライヤ適用
  - _Requirements: 16.1_

- [ ] 12.3 ネッティングセット処理とEAD算出
  - alpha係数(1.4)適用
  - 担保契約の影響
  - 最終EAD = alpha * (RC + PFE)
  - _Requirements: 16.2_

- [ ] 13. FRTBエンジン実装
- [ ] 13.1 Delta感応度計算
  - リスククラス別(GIRR, CSR, EQ, COMM, FX)
  - バケット分類
  - 感応度計算方法
  - _Requirements: 16.3_

- [ ] 13.2 (P) Vega感応度計算
  - オプションボラティリティ感応度
  - テナー・マネーネス軸
  - バケット内集約
  - _Requirements: 16.3_

- [ ] 13.3 (P) Curvature感応度計算
  - 非線形リスク捕捉
  - シフト幅の適用
  - Curvatureチャージ計算
  - _Requirements: 16.3_

- [ ] 13.4 資本チャージ計算(Delta/Vega/Curvature)
  - バケット内相関適用
  - バケット間相関適用
  - 3つの相関シナリオ対応
  - _Requirements: 16.3_

- [ ] 14. SIMMエンジン実装
- [ ] 14.1 リスククラス別マージン計算
  - Interest Rate, CreditQ, CreditNonQ, Equity, Commodity, FX
  - 感応度入力形式
  - バケット・テナー分類
  - _Requirements: 16.4_

- [ ] 14.2 バケット内・バケット間集約
  - 相関行列の適用
  - 平方根和による集約
  - 濃度リスク調整
  - _Requirements: 16.4_

- [ ] 14.3 プロダクトクラス集約
  - RatesFX, Credit, Equity, Commodity分類
  - プロダクトクラス間相関
  - 最終IM算出
  - _Requirements: 16.4_

- [ ] 15. 規制レポートと監査証跡
- [ ] 15.1 規制計算の監査証跡記録
  - 入力パラメータのログ
  - 中間計算結果の保存
  - 計算ステップのトレーサビリティ
  - _Requirements: 16.6_

- [ ] 15.2 (P) Basel III/IV資本要件計算
  - 信用リスク資本
  - 市場リスク資本
  - 統合資本比率
  - _Requirements: 16.7_

- [ ] 15.3 (P) 規制レポート生成
  - 所定フォーマット対応
  - XML/CSV出力
  - 期間比較レポート
  - _Requirements: 16.8_

- [ ] 15.4 IMMエクスポージャー計算
  - 内部モデル法対応
  - シミュレーションベースEPE
  - 規制承認パラメータ
  - _Requirements: 16.2_

## Phase 5: マーケットデータ・キャリブレーション強化

- [ ] 16. マーケットデータ基盤拡張
- [ ] 16.1 ボラティリティ曲面構築(SSVI/SVI)
  - SSVI(Surface SVI)パラメータ化
  - 無裁定条件の強制
  - スライス間整合性
  - _Requirements: 11.2_

- [ ] 16.2 (P) クレジットカーブ構築
  - CDSスプレッドからの構築
  - ハザードレートブートストラップ
  - 回収率の取り扱い
  - _Requirements: 11.4_

- [ ] 16.3 (P) コモディティフォワードカーブ(季節性対応)
  - 季節性パターンの抽出
  - フォワード価格の補間
  - コンビニエンスイールド曲線
  - _Requirements: 11.3_

- [ ] 16.4 マーケットデータ無裁定検証
  - カレンダースプレッド非負
  - バタフライ非負
  - ボラティリティ曲面の一般化条件
  - _Requirements: 11.7_

- [ ] 16.5 マーケットデータキャッシュ無効化
  - 依存関係グラフ管理
  - 更新時の自動無効化
  - 遅延再計算
  - _Requirements: 11.5_

- [ ] 16.6 AD互換マーケットデータ構造
  - カーブ感応度計算対応
  - ジェネリック型`T: Float`維持
  - AD用ラッパー
  - _Requirements: 11.6_

- [ ] 16.7 マーケットデータ検証エラーメッセージ
  - 詳細なエラー情報
  - 修正提案の提示
  - 構造化エラー型
  - _Requirements: 11.8_

- [ ] 17. キャリブレーション機能強化
- [ ] 17.1 グローバル最適化(微分進化/焼きなまし)
  - 微分進化アルゴリズム
  - 焼きなまし法
  - 非凸問題対応
  - _Requirements: 12.7_

- [ ] 17.2 キャリブレーション品質メトリクス
  - RMSE, 最大誤差
  - ビッド・アスクカバレッジ
  - 収束診断
  - _Requirements: 12.8_

- [ ] 17.3 Hull-Whiteキャリブレーション(Cap/Floor/Swaption)
  - 平均回帰速度・ボラティリティ推定
  - Swaption曲面へのフィット
  - Cap/Floorプライスへのフィット
  - _Requirements: 12.5_

- [ ] 17.4 キャリブレーション失敗時の診断情報
  - パラメータ境界の提示
  - 収束履歴
  - 推奨アクション
  - _Requirements: 12.6_

## Phase 6: Greeks・パフォーマンス最適化

- [ ] 18. Greeks計算の高度化
- [ ] 18.1 二次Greeks(Vanna, Volga, Charm, Vomma)完成
  - 各二次Greeksの定義と計算
  - Bump-and-revalue法での実装
  - AAD法との整合性検証
  - _Requirements: 10.2_

- [ ] 18.2 クロスガンマ(多因子モデル)
  - 複数リスクファクター間の交差項
  - 相関モデルでのクロスガンマ
  - 行列形式での出力
  - _Requirements: 10.3_

- [ ] 18.3 Pathwise Greeks(MC用)
  - パスワイズ微分法
  - ペイオフ微分の伝播
  - 不連続ペイオフ対応(Likelihood Ratio)
  - _Requirements: 10.7, 10.8_

- [ ] 18.4 Greeks検証フレームワーク
  - 有限差分近似との比較
  - AAD vs Bump-and-revalue比較
  - 自動回帰テスト
  - _Requirements: 10.9_

- [ ] 19. パフォーマンス最適化
- [ ] 19.1 バッチプライシング(10,000商品/秒)
  - ポートフォリオレベル並列化
  - Rayonによるマルチコア活用
  - ワークスチール最適化
  - _Requirements: 14.4, 14.5_

- [ ] 19.2 (P) SIMD ベクトル化(パス生成)
  - packed_simd活用
  - 正規乱数のベクトル生成
  - パス演算のSIMD化
  - _Requirements: 14.7_

- [ ] 19.3 (P) メモリアロケーション最小化
  - 事前割り当てバッファ拡張
  - ThreadLocalPoolの最適化
  - ホットパスでのゼロアロケーション
  - _Requirements: 14.6_

- [ ] 19.4 パフォーマンスベンチマーク・回帰テスト
  - Criterionベンチマーク整備
  - CI/CDでの回帰検出
  - ターゲットメトリクス監視(バニラ<10us、MC 100kパス<100ms)
  - _Requirements: 14.1, 14.2, 14.3, 14.8_

## Phase 7: 本番運用信頼性

- [ ] 20. エラーハンドリング・信頼性
- [ ] 20.1 構造化エラー型の完成
  - 全モジュール用エラー型定義
  - thiserrorによる実装
  - エラーチェーン対応
  - _Requirements: 13.2, 17.8_

- [ ] 20.2 数値オーバーフロー/アンダーフロー処理
  - NaN/Infinity検出とフラグ
  - 安全なデフォルト動作
  - エラーコンテキスト保存
  - _Requirements: 13.3_

- [ ] 20.3 決定論的リプレイ(シード管理)
  - 設定可能な乱数シード
  - 再現可能なシミュレーション
  - シード管理API
  - _Requirements: 13.4_

- [ ] 20.4 (P) バックプレッシャーとSLA管理
  - 高負荷時のレスポンスタイム維持
  - キュー管理
  - グレースフルデグラデーション
  - _Requirements: 13.5_

- [ ] 20.5 (P) ヘルスチェック・リブネスプローブ
  - コンテナオーケストレーション対応
  - ヘルスエンドポイント
  - 依存サービスチェック
  - _Requirements: 13.7_

- [ ] 20.6 構造化ログ(tracing)
  - 設定可能な冗長性レベル
  - 構造化フィールド
  - スパン・イベント対応
  - _Requirements: 13.6_

- [ ] 20.7 実験機能(Enzyme)の隔離
  - フィーチャーフラグによる隔離
  - 安定プロダクションパスからの分離
  - フォールバック機構
  - _Requirements: 13.8_

## Phase 8: API・インテグレーション

- [ ] 21. APIレイヤー実装
- [ ] 21.1 Rust ネイティブAPI(ゼロコスト抽象)
  - パブリックAPI設計
  - ビルダーパターン適用
  - ドキュメントコメント
  - _Requirements: 17.1, 17.6_

- [ ] 21.2 (P) C FFIバインディング
  - extern "C"インターフェース
  - メモリ安全性確保
  - ヘッダファイル生成
  - _Requirements: 17.2_

- [ ] 21.3 (P) Pythonバインディング(PyO3)
  - PyO3による型変換
  - Numpy連携
  - 非同期対応(pyo3-asyncio)
  - _Requirements: 17.3_

- [ ] 21.4 非同期バッチ操作サポート
  - async/await対応
  - ノンブロッキングバッチ処理
  - Tokio/async-std対応
  - _Requirements: 17.4_

- [ ] 21.5 (P) enum静的ディスパッチ(AD互換)
  - Box<dyn Trait>回避
  - Enzyme最適化対応
  - 型安全なディスパッチ
  - _Requirements: 17.7_

- [ ] 21.6 後方互換性維持
  - セマンティックバージョニング
  - 非推奨APIの管理
  - 移行ガイド
  - _Requirements: 17.5_

## Phase 9: テスト・検証強化

- [ ] 22. テストフレームワーク拡充
- [ ] 22.1 プロパティベーステスト(数学的不変条件)
  - Put-Call Parity検証
  - 単調性・凸性条件
  - 無裁定条件
  - _Requirements: 18.2_

- [ ] 22.2 (P) ベンチマークテスト(パフォーマンス回帰検出)
  - Criterionベンチマークスイート
  - iai-callgrind(命令数ベース)
  - CIでの回帰検出
  - _Requirements: 18.3_

- [ ] 22.3 MC vs 解析解検証テスト
  - ヨーロピアンオプションでの比較
  - 許容誤差(1%以内)検証
  - 収束率テスト
  - _Requirements: 18.4_

- [ ] 22.4 Enzyme vs num-dual検証テスト
  - AD結果の整合性
  - 許容誤差検証
  - 回帰テスト
  - _Requirements: 18.5_

- [ ] 22.5 (P) ファジングテスト(入力検証ロバスト性)
  - cargo-fuzz活用
  - 入力境界のテスト
  - パニック検出
  - _Requirements: 18.6_

- [ ] 22.6 (P) 統合テスト(リアルマーケットデータシナリオ)
  - 実市場データでのE2Eテスト
  - ストレスシナリオ
  - シナリオベースバリデーション
  - _Requirements: 18.7_

- [ ] 22.7 コードカバレッジ(80%以上)
  - tarpaulin/lcovsによるカバレッジ測定
  - CI/CDでの閾値チェック
  - カバレッジレポート生成
  - _Requirements: 18.8_

## Phase 10: デプロイメント・運用基盤

- [ ] 23. デプロイメント基盤
- [ ] 23.1 Dockerイメージ(stable/nightly)
  - マルチステージビルド
  - stable/nightlyの分離
  - 軽量イメージ最適化
  - _Requirements: 20.1_

- [ ] 23.2 (P) フィーチャーフラグ(アセットクラス有効/無効)
  - コンパイル時フラグ
  - ランタイム設定オプション
  - アセットクラス別切り替え
  - _Requirements: 20.2_

- [ ] 23.3 (P) 環境変数/設定ファイル対応
  - 12-Factor App準拠
  - TOML/YAML設定
  - 環境別オーバーライド
  - _Requirements: 20.3_

- [ ] 23.4 Kubernetesスケーリング対応
  - 水平スケーリング
  - Pod affinity/anti-affinity
  - リソースリクエスト/リミット
  - _Requirements: 20.4_

- [ ] 23.5 (P) Prometheusメトリクスエクスポート
  - カスタムメトリクス定義
  - ヒストグラム・カウンター
  - Grafanaダッシュボード
  - _Requirements: 20.5_

- [ ] 23.6 (P) 分散トレーシング(OpenTelemetry)
  - トレースコンテキスト伝播
  - スパン計装
  - Jaeger/Zipkin連携
  - _Requirements: 20.6_

- [ ] 23.7 再現可能ビルド
  - ビルドアーティファクトの一貫性
  - 依存バージョン固定
  - ビルドメタデータ埋め込み
  - _Requirements: 20.7_

- [ ] 23.8 ゼロダウンタイムローリングアップデート
  - Readinessプローブ
  - グレースフルシャットダウン
  - ローリング戦略
  - _Requirements: 20.8_

## Phase 11: 最終統合と品質保証

- [ ] 24. システム統合テスト
- [ ] 24.1 エンドツーエンド価格計算フロー検証
  - マーケットデータ→キャリブレーション→プライシング→Greeks
  - 全アセットクラス横断
  - 本番シナリオ再現
  - _Requirements: 1.1, 2.1, 3.1, 4.1, 5.1_

- [ ] 24.2 規制計算フロー統合テスト
  - SA-CCR→FRTB→SIMM連携
  - ポートフォリオレベルXVA
  - レポート生成確認
  - _Requirements: 16.1, 16.3, 16.4_

- [ ] 24.3 パフォーマンス総合検証
  - 全ターゲットメトリクス達成確認
  - 負荷テスト
  - 長時間安定性テスト
  - _Requirements: 14.1, 14.2, 14.3, 14.8_

- [ ] 24.4 可用性・信頼性検証(99.99%)
  - 障害注入テスト
  - リカバリ時間測定
  - グレースフルデグラデーション確認
  - _Requirements: 13.1_
