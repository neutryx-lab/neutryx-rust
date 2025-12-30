# Implementation Plan

## Tasks

- [ ] 1. Smooth Approximation関数の拡張（L1: pricer_core）
- [x] 1.1 (P) smooth_sqrt関数の実装
  - ゼロ近傍で滑らかに動作する平方根関数を実装
  - 数学的定義: smooth_sqrt(x, epsilon) = sqrt(x + epsilon^2) - epsilon
  - ジェネリックFloat型でf32、f64、DualNumber型をサポート
  - epsilonが小さい場合に真の平方根値に収束することを検証するテスト
  - _Requirements: 8.1, 8.4, 8.5_

- [x] 1.2 (P) smooth_log関数の実装
  - ゼロ近傍で滑らかに動作する対数関数を実装
  - smooth_max関数と組み合わせて特異点を回避
  - ジェネリックFloat型でf32、f64、DualNumber型をサポート
  - log-sum-exp技法による数値オーバーフロー防止
  - _Requirements: 8.2, 8.4, 8.5, 8.6_

- [x] 1.3 (P) smooth_pow関数の実装
  - 非整数べき乗を滑らかに処理するべき乗関数を実装
  - smooth_log、smooth_absとの組み合わせでAD互換性を維持
  - ジェネリックFloat型でf32、f64、DualNumber型をサポート
  - 安定化技法を使用して数値オーバーフローを防止
  - _Requirements: 8.3, 8.4, 8.5, 8.6_

- [x] 2. StochasticModelトレイト抽象化（L2: pricer_models）
- [x] 2.1 StochasticModelトレイトの定義
  - 確率過程モデルの統一インターフェースを定義
  - ジェネリック型パラメータT: Floatを使用してf64およびDualNumber型をサポート
  - evolve_step、initial_state、brownian_dimメソッドを定義
  - 関連型State、Paramsの定義
  - Differentiableマーカートレイトの要求を含める
  - 静的ディスパッチ（enum-based）のみをサポートし、Box<dyn Trait>は禁止
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6_

- [x] 2.2 StochasticModelEnumの定義
  - GBM、Heston、SABRをenumバリアントとしてラップ
  - match式によるゼロコスト抽象化を実装
  - model_name、brownian_dim、is_two_factorメソッドを提供
  - Task 2.1完了後、HestonModelとSABRModelの実装完了後に統合
  - _Requirements: 1.6, 7.1, 7.2_

- [ ] 3. Hestonモデル実装（L2: pricer_models）
- [ ] 3.1 HestonParams構造体とエラー型の定義
  - スポット価格、初期分散、長期分散、平均回帰速度、xi、rho、リスクフリーレート、満期を含むパラメータ構造体
  - QE切り替え閾値psi_cとsmoothing_epsilonパラメータ
  - HestonError列挙型（InvalidSpot、InvalidV0、InvalidRho等）を定義
  - thiserrorクレートによる構造化エラー
  - _Requirements: 2.1, 10.1, 10.3, 10.4, 10.5_

- [ ] 3.2 HestonModelの基本実装
  - HestonParams構造体を内包するHestonModel構造体を作成
  - new、validateメソッドを実装
  - Feller条件チェック（2*kappa*theta > xi^2）を実装
  - 条件違反時の警告ログ出力と分散フロア適用
  - ジェネリックFloat型で実装
  - Task 3.1のエラー型を使用
  - _Requirements: 2.2, 2.5, 2.6, 10.1, 10.6_

- [ ] 3.3 QE離散化スキームの実装
  - Andersen (2008) QE離散化スキームを実装
  - psi値に基づく二次スキームと指数スキームの切り替え
  - smooth_indicatorによる滑らかなスキーム切り替え
  - smooth_maxによる分散の非負性保証
  - 相関ブラウン運動生成（Cholesky分解）
  - Task 1.1のsmooth_sqrt使用
  - _Requirements: 2.2, 2.3, 2.4_

- [ ] 3.4 StochasticModelトレイト実装
  - HestonModelにStochasticModelトレイトを実装
  - State型を(T, T)（価格、分散）として定義
  - evolve_step、initial_state、brownian_dimを実装
  - Differentiableマーカートレイトを実装
  - Task 2.1、3.2、3.3の完了が必要
  - _Requirements: 2.6, 2.7, 1.5_

- [ ] 4. SABRモデル実装（L2: pricer_models）
- [ ] 4.1 (P) SABRParams構造体とエラー型の定義
  - フォワード価格、alpha、nu、rho、betaを含むパラメータ構造体
  - 満期、ATM近傍判定閾値、smoothing_epsilonパラメータ
  - SABRError列挙型（InvalidAlpha、InvalidBeta、InvalidRho等）を定義
  - thiserrorクレートによる構造化エラー
  - _Requirements: 3.1, 10.2, 10.5_

- [ ] 4.2 Haganインプライドボラティリティ公式の実装
  - 任意のストライクに対するインプライドボラティリティ計算
  - ATM近傍での展開公式による数値安定性確保
  - smooth_log、smooth_powを使用したAD互換性維持
  - z/x(z)係数と高次展開項の実装
  - Task 1.2、1.3、4.1の完了が必要
  - _Requirements: 3.2, 3.3, 3.7_

- [ ] 4.3 特殊ケース処理とパラメータ検証
  - beta=0（Normal SABR）モードの実装
  - beta=1（Lognormal SABR）モードの実装
  - パラメータ検証（alpha > 0、0 <= beta <= 1、|rho| < 1）
  - 負のインプライドボラティリティに対するフロア適用
  - ジェネリックFloat型で実装
  - _Requirements: 3.4, 3.5, 3.6, 10.2, 10.3, 10.4_

- [ ] 4.4 Differentiableマーカートレイト実装
  - SABRModelにDifferentiableマーカートレイトを実装
  - implied_vol、atm_volメソッドのResult型による戻り値
  - Task 4.2、4.3の完了が必要
  - _Requirements: 3.4, 1.5_

- [ ] 5. EnzymeContext実装（L3: pricer_kernel）
- [ ] 5.1 ADMode列挙型とActivity列挙型の定義
  - ADMode列挙型（Inactive、Forward、Reverse）を定義
  - Activity列挙型（Const、Dual、Active、Duplicated、DuplicatedOnly）を定義
  - 各列挙値にClone、Copy、Debug、PartialEq、Eq、Hash派生を実装
  - _Requirements: 4.1, 4.3, 4.4, 4.6_

- [ ] 5.2 EnzymeContext構造体の実装
  - 現在のADモードを保持するフィールド
  - パラメータ名とActivityのマッピング（HashMap）
  - new、current_mode、set_mode、set_activity、get_activityメソッド
  - is_forward、is_reverseヘルパーメソッド
  - _Requirements: 4.2, 4.3, 4.4, 4.6_

- [ ] 5.3 スレッドローカルストレージの実装
  - thread_local!マクロによるスレッドローカルコンテキスト
  - with_enzyme_context、with_enzyme_context_mut関数
  - マルチスレッド環境での安全な使用保証
  - _Requirements: 4.5_

- [ ] 5.4 有限差分フォールバックの実装
  - Enzyme完全統合前の有限差分による勾配計算
  - Forward mode、Reverse modeの両方をサポート
  - 将来のEnzyme統合に備えた設計
  - _Requirements: 4.7_

- [ ] 6. Tangentパス生成（L3: pricer_kernel）
- [ ] 6.1 HestonTangentSeeds構造体の定義
  - 各Hestonパラメータに対するtangent seed（d_spot、d_v0、d_kappa、d_theta、d_xi、d_rho、d_rate）
  - Defaultトレイト実装
  - _Requirements: 5.2_

- [ ] 6.2 HestonWorkspaceの拡張
  - tangentパス用のバッファ追加（tangent_prices、tangent_variances）
  - 容量確保メソッドの拡張
  - Task 3.4のHestonModelと統合
  - _Requirements: 5.4_

- [ ] 6.3 generate_heston_paths_tangent関数の実装
  - Primalパスとtangentパスの同時生成
  - 各パラメータに対するtangent seed入力の処理
  - 相関ブラウン運動のtangent伝播
  - EnzymeContext.mode == Forwardの事前条件チェック
  - Task 5.1-5.3、6.1、6.2の完了が必要
  - _Requirements: 5.1, 5.3, 5.4, 5.5, 5.6_

- [ ] 7. Adjointパス生成（L3: pricer_kernel）
- [ ] 7.1 CheckpointStrategy列挙型の定義
  - StoreAll、Recompute、Binomialバリアントを定義
  - estimate_memory、autoメソッドの実装
  - メモリ使用量の推定ロジック
  - _Requirements: 6.4, 6.5_

- [ ] 7.2 HestonAdjointResult構造体の定義
  - 各パラメータに対するadjoint値（adj_spot、adj_v0等）
  - Defaultトレイト実装
  - _Requirements: 6.3_

- [ ] 7.3 generate_heston_paths_adjoint関数の実装
  - 終端payoffからの感度逆伝播
  - チェックポインティングによるメモリ制御
  - 全パラメータ感度の一括計算
  - EnzymeContext.mode == Reverseの事前条件チェック
  - Enzyme #[autodiff_reverse]マクロへの将来的な移行を考慮
  - Task 5.1-5.3、7.1、7.2の完了が必要
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.6_

- [ ] 7.4 メモリ不足時の再計算フォールバック
  - CheckpointStrategy.autoによる自動選択
  - メモリ超過時のRecompute戦略へのフォールバック
  - エラーハンドリングとログ出力
  - _Requirements: 6.5_

- [ ] 8. MonteCarloPricerとの統合（L3: pricer_kernel）
- [ ] 8.1 PricingResultAD構造体とHestonGreeks構造体の定義
  - 基本PricingResultの拡張
  - Heston固有のGreeks（vega_v0、vega_theta、kappa_sensitivity、xi_sensitivity、rho_sensitivity）
  - _Requirements: 7.6_

- [ ] 8.2 price_european_hestonメソッドの実装
  - HestonParams入力によるHestonモデル価格計算
  - 2次元パス（資産価格、分散）の生成
  - 一貫したPricingResult構造体の返却
  - Task 3.4のHestonModel、Task 2.2のStochasticModelEnumが必要
  - _Requirements: 7.1, 7.2, 7.6_

- [ ] 8.3 price_with_greeks_adメソッドの実装
  - StochasticModelEnumによる任意モデルサポート
  - ADModeに基づくForward/Reverse mode切り替え
  - tangent/adjointパスを使用したGreeks計算
  - モデル固有設定（QEパラメータ等）の受け取り
  - Task 6.3、7.3、8.1、8.2の完了が必要
  - _Requirements: 7.3, 7.4, 7.5, 7.6_

- [ ] 9. デュアルモード検証（L3: pricer_kernel）
- [ ] 9.1 verifyモジュールの拡張
  - Heston/SABRモデル用の検証テストを追加
  - num-dual DualNumber型による参照勾配計算
  - 複数のパラメータセット（ATM、ITM、OTM）をカバー
  - _Requirements: 9.1, 9.2, 9.5_

- [ ] 9.2 勾配比較検証関数の実装
  - Enzyme（または有限差分フォールバック）とnum-dual勾配の比較
  - 相対誤差閾値（デフォルト: 1e-4）による判定
  - 閾値超過時の詳細エラーメッセージ出力
  - _Requirements: 9.3, 9.4_

- [ ] 9.3 CI/CDパイプライン統合
  - 検証テストの自動実行設定
  - テスト結果のレポート生成
  - 失敗時のビルド中断設定
  - _Requirements: 9.6_

- [ ] 10. エラーハンドリングと境界条件（L2/L3統合）
- [ ] 10.1 境界条件処理の実装
  - T=0（満期ゼロ）での適切な処理
  - S=K（ATM）での特異点回避
  - NaN/Infinity検出と早期リターン
  - 明確なエラーメッセージの提供
  - Task 3.2、4.3のエラー型を使用
  - _Requirements: 10.3, 10.4, 10.6_

- [ ] 10.2 数値安定性検証テスト
  - 極端なパラメータ値でのモデル動作検証
  - NumericalInstabilityエラーの適切な発生確認
  - 境界条件でのエラーメッセージ検証
  - _Requirements: 10.3, 10.4, 10.5, 10.6_

- [ ] 11. 統合テストと最終検証
- [ ] 11.1 Hestonモデル統計テスト
  - パス生成の平均・分散統計テスト
  - 解析的モーメントとの比較
  - QE離散化精度の検証
  - _Requirements: 2.2, 2.4_

- [ ] 11.2 SABRインプライドボラティリティテスト
  - 文献の既知値との比較テスト
  - スマイル形状の検証
  - ATM/ITM/OTM各領域での精度確認
  - _Requirements: 3.2, 3.3_

- [ ] 11.3 MonteCarloPricer統合テスト
  - GBM、Heston、SABRモデルの統一API動作確認
  - price_with_greeks_adの全モード動作検証
  - 一貫したPricingResult構造体の検証
  - _Requirements: 7.1, 7.4, 7.5, 7.6_

- [ ] 11.4 ADベースGreeks精度テスト
  - bump-and-revalueとtangentパスの比較
  - num-dualとadjointパスの比較
  - Delta、Vega等の主要Greeksの精度検証
  - _Requirements: 5.3, 6.2, 9.3, 9.4_
