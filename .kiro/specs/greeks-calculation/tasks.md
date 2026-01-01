# Implementation Plan

## Task Overview

本実装計画では、Greeks計算機能を段階的に構築する。既存のMonteCarloPricerを拡張し、一次・二次Greeks、複数計算モード、およびポートフォリオレベル集約をサポートする。

---

## Tasks

- [ ] 1. 基盤型の定義
- [ ] 1.1 (P) Greeks結果構造体の実装
  - 一次Greeks（delta, vega, theta, rho）と二次Greeks（gamma, vanna, volga）を保持するジェネリック構造体を作成
  - 価格と標準誤差フィールドを含め、Option型で未計算Greeksを表現
  - T: Float トレイト境界を設定してAD互換性を確保
  - 信頼区間計算用のヘルパーメソッドを追加（95%、99%）
  - serde feature有効時のSerialize/Deserialize対応を追加
  - 既存PricingResultを新構造体のtype aliasとして再定義
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

- [ ] 1.2 (P) Greek enum拡張とGreeksConfig実装
  - 既存Greek enumにVanna、Volgaバリアントを追加
  - 計算モード（BumpRevalue、EnzymeAAD、NumDual）を表すenum作成
  - バンプ幅設定（spot相対、vol絶対、time、rate）を保持する構造体を作成
  - 検証許容誤差のデフォルト値（1e-6）を設定
  - Builder patternによる設定構築をサポート
  - バリデーションロジック（正のバンプ幅、合理的な範囲）を実装
  - _Requirements: 3.3, 5.4_

- [ ] 2. Bump-and-Revalue計算エンジン
- [ ] 2.1 一次Greeksのbump計算リファクタリング
  - 既存のcompute_delta/vega/theta/rho関数をGreeksConfigのバンプ幅を使用するよう修正
  - シード復元による再現性確保のロジックを共通化
  - 中心差分法の実装を検証（(f(x+h) - f(x-h)) / 2h）
  - 各Greekの単体テストを追加
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 3.1, 3.3, 3.4_

- [ ] 2.2 二次Greeks（Gamma, Vanna, Volga）の実装
  - Gamma計算の三点差分法を検証（(f(x+h) - 2f(x) + f(x-h)) / h²）
  - Vanna計算のクロス差分実装（∂²V/∂S∂σ）
  - Volga計算の三点差分実装（∂²V/∂σ²）
  - smooth近似によるATM近傍での数値安定性確保
  - 二次Greeks計算の単体テスト追加
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 3.2_

- [ ] 2.3 BumpRevalueCalculatorの統合
  - pricer参照とconfig参照を保持するCalculator構造体を作成
  - 全Greeks計算メソッドをCalculatorに集約
  - ジェネリック型対応（f64固定からT: Float対応への準備）
  - Calculatorの単体テストと統合テストを追加
  - _Requirements: 1.5, 3.1, 3.2, 3.3, 3.4_

- [ ] 3. GreeksEngine統合層
- [ ] 3.1 GreeksEngine構造体の実装
  - MonteCarloPricerへの参照とGreeksConfigを保持
  - 計算モードに応じたCalculator選択ロジックを実装
  - compute_european: 選択されたGreeksのみを計算するメソッド
  - compute_all: 全Greeksを一括計算するメソッド
  - 入力パラメータ（GbmParams、PayoffParams）のバリデーション
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 2.1, 2.2, 2.3, 2.4_

- [ ] 3.2 MonteCarloPricerへの統合
  - price_with_greeks_configメソッドの追加（GreeksConfig対応版）
  - 既存price_with_greeksとの後方互換性維持
  - デフォルトGreeksConfigの適用
  - 統合テスト：ATMコールのDelta≈0.5検証
  - put-call parityテスト：Delta(Call) - Delta(Put) ≈ 1
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 3.1_

- [ ] 4. パス依存オプションのGreeks
- [ ] 4.1 (P) パス依存Greeks計算の拡張
  - 既存price_path_dependent_with_greeksをGreeksConfig対応に拡張
  - Asianオプション用Delta/Vega計算の検証
  - Barrierオプション用smooth近似Greeksの実装
  - Lookbackオプション用Greeks計算の検証
  - _Requirements: 7.1, 7.2, 7.3_

- [ ] 4.2 (P) PathDependentPayoffトレイト対応
  - PathDependentPayoffを実装した任意のペイオフに対するGreeks計算
  - PathObserverのストリーミング統計を活用した効率的計算
  - パス依存Greeks計算の単体テスト追加
  - _Requirements: 7.4_

- [ ] 5. num-dual検証エンジン
- [ ] 5.1 num-dualモードGreeks計算
  - num-dual-mode feature flagで有効化される計算パス
  - Dual<f64>を使用したforward-mode AD Delta計算
  - HyperDualを使用した二次Greeks計算
  - BumpRevalueとの結果比較テスト
  - _Requirements: 5.1_

- [ ] 5.2 検証エンジンの実装
  - 複数計算モード結果の比較機能
  - 許容誤差閾値による合格/不合格判定
  - 差異レポート出力（絶対誤差、相対誤差）
  - 検証テストスイートの作成
  - _Requirements: 5.2, 5.3, 5.4_

- [ ] 6. Enzyme AAD統合（feature-flagged）
- [ ] 6.1 Enzyme AADインフラストラクチャ
  - enzyme-mode feature flagの設定
  - #[autodiff]マクロを使用した微分可能関数の定義
  - GBMパス生成のAD対応
  - ペイオフ計算のAD対応
  - _Requirements: 4.1, 4.3_

- [ ] 6.2 AADCalculatorの実装
  - Enzyme reverse-mode ADによる全Greeks計算
  - BumpRevalueと同等精度の検証
  - Enzymeコンパイルエラー時のエラーハンドリング
  - フォールバック手段（BumpRevalueへの切り替え）の実装
  - _Requirements: 4.1, 4.2, 4.3, 4.4_

- [ ] 7. ポートフォリオレベルGreeks（L4）
- [ ] 7.1 PortfolioGreeks構造体の実装
  - pricer_xvaクレートにPortfolioGreeksモジュールを追加
  - Trade traitを実装したオブジェクトのGreeks計算
  - Rayonによる並列Greeks計算
  - _Requirements: 8.1, 8.2_

- [ ] 7.2 ネッティングセット集約
  - ネッティングセットIDによるGreeksグループ化
  - 同一ネッティングセット内Greeksの合算
  - SoA形式でのメモリ効率化（大規模ポートフォリオ対応）
  - _Requirements: 8.3, 8.4_

- [ ] 8. 統合テストとベンチマーク
- [ ] 8.1 統合テストスイート
  - GreeksEngine全機能の統合テスト
  - European/Path-dependent両方のGreeks計算テスト
  - 数学的不変量テスト（put-call parity、Delta bounds等）
  - proptestによるプロパティベーステスト
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3_

- [ ] 8.2 パフォーマンスベンチマーク
  - criterionによるGreeks計算ベンチマーク
  - 10,000パスでの計算時間測定
  - ポートフォリオ並列計算のスケーラビリティ測定
  - BumpRevalue vs AADのパフォーマンス比較
  - _Requirements: 4.2, 8.2_

---

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1.1, 1.2, 1.3, 1.4 | 2.1, 3.1, 3.2, 8.1 |
| 1.5 | 2.3 |
| 2.1, 2.2, 2.3, 2.4 | 2.2, 3.1, 8.1 |
| 3.1, 3.2 | 2.1, 2.2, 2.3 |
| 3.3 | 1.2, 2.1, 2.3 |
| 3.4 | 2.1, 2.3 |
| 4.1, 4.2, 4.3, 4.4 | 6.1, 6.2 |
| 5.1 | 5.1 |
| 5.2, 5.3, 5.4 | 5.2 |
| 6.1, 6.2, 6.3, 6.4, 6.5 | 1.1 |
| 7.1, 7.2, 7.3 | 4.1 |
| 7.4 | 4.2 |
| 8.1, 8.2 | 7.1 |
| 8.3, 8.4 | 7.2 |
