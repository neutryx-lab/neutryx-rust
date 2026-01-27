# Implementation Plan

## Task Overview

本実装計画はGlobal Curve Builderの段階的な実装を定義する。A-I-P-Sアーキテクチャに従い、L1（pricer_core）からL4（pricer_risk）へと積み上げ式に実装を進める。

---

## Tasks

### Phase 1: L1 Foundation (pricer_core)

- [ ] 1. ソルバー基盤型の実装
- [ ] 1.1 (P) SolverError列挙型の拡張
  - 多次元ソルバー固有のエラーバリアントを追加
  - `SingularJacobian`（最小ピボット値を含む）を実装
  - `DimensionMismatch`（期待次元と実際次元を含む）を実装
  - 既存バリアントとの互換性を維持
  - _Requirements: 9_
  - _Contracts: SolverError拡張_

- [ ] 1.2 (P) SystemOfEquationsトレイトの定義
  - 連立方程式の統一インターフェースを設計
  - `dimension()`, `evaluate()`, `jacobian()` メソッドを定義
  - 数値Jacobianのデフォルト実装を提供（有限差分法）
  - ジェネリック型パラメータ `T: RealField + Copy` を採用
  - _Requirements: 2_
  - _Contracts: SystemOfEquations Trait_

- [ ] 1.3 (P) SolverResult構造体の実装
  - 解ベクトル、残差ノルム、反復回数を格納
  - Jacobian逆行列のオプション格納機能を実装
  - 収束フラグとAAD用メタデータを含む
  - Clone, Debug トレイト実装
  - _Requirements: 3_
  - _Contracts: SolverResult State_

### Phase 2: L1 Solver Implementation (pricer_core)

- [ ] 2. 多次元Newton-Raphsonソルバーの実装
- [ ] 2.1 NewtonConfig設定構造体の実装
  - 収束許容誤差（残差ノルム、パラメータ変化量）を設定可能に
  - 最大反復回数の設定
  - 数値Jacobianのイプシロン設定
  - Default, fast, high_precision プリセット実装
  - _Requirements: 1_

- [ ] 2.2 Newton-Raphson反復アルゴリズムの実装
  - x_{k+1} = x_k - J(x_k)⁻¹ F(x_k) の反復を実装
  - 既存linalg::lu_solveを使用した線形システム求解
  - 収束判定（残差ノルムおよびパラメータ変化量の二重条件）
  - 特異Jacobian検出とエラー返却
  - _Requirements: 1, 7_
  - _Contracts: MultidimensionalNewtonSolver Service_

- [ ] 2.3 Jacobian逆行列の計算と格納
  - 収束時にlinalg::inverseを使用してJacobian逆行列を計算
  - SolverResultへの逆行列格納（AAD用）
  - 遅延計算オプション（store_jacobian_inverse フラグ）
  - _Requirements: 1, 3, 7_

### Phase 3: L2 Calibration Infrastructure (pricer_models)

- [ ] 3. キャリブレーション商品インターフェースの実装
- [ ] 3.1 CalibrationInstrumentトレイトの定義
  - 残差計算メソッド `residual(curve)` を定義
  - Jacobian行寄与計算 `jacobian_row(curve, epsilon)` を定義
  - 満期取得メソッド（ソート用）を定義
  - ジェネリックカーブ型パラメータ対応
  - _Requirements: 5_
  - _Contracts: CalibrationInstrument Trait_

- [ ] 3.2 既存BootstrapInstrumentへのトレイト実装
  - OIS, IRS, FRA, Futureの各バリアントにCalibrationInstrument実装
  - 既存のresidual()メソッドを活用
  - jacobian_row()の数値微分実装
  - _Requirements: 5_

### Phase 4: L2 Curve Calibration Problem (pricer_models)

- [ ] 4. カーブキャリブレーション問題の実装
- [ ] 4.1 CurveCalibrationProblem構造体の実装
  - キャリブレーション商品群とカーブテンプレートを保持
  - カーブノードベクトルからカーブオブジェクトを再構築するbuild_curve()実装
  - 商品の満期順ソート機能
  - _Requirements: 4_
  - _Contracts: CurveCalibrationProblem Service_

- [ ] 4.2 SystemOfEquationsトレイト実装
  - evaluate(): 各商品の残差をベクトルとして収集
  - jacobian(): 各商品のjacobian_rowを行として構築
  - dimension(): 商品数（=カーブノード数）を返却
  - _Requirements: 4_

### Phase 5: L2 GlobalBootstrapper (pricer_models)

- [ ] 5. グローバルブートストラッパーの実装
- [ ] 5.1 GlobalBootstrapper構造体と設定の実装
  - GlobalBootstrapConfig（ソルバー設定、Jacobian逆行列格納フラグ）
  - BootstrapResult（カーブ、反復回数、収束フラグ、Jacobian逆行列）
  - コンストラクタとcalibrateメソッドのシグネチャ実装
  - _Requirements: 8_
  - _Contracts: GlobalBootstrapper Service_

- [ ] 5.2 calibrateメソッドの実装
  - CurveCalibrationProblemの構築
  - MultidimensionalNewtonSolverの呼び出し
  - SolverResultからBootstrapResultへの変換
  - エラーハンドリングとログ出力
  - _Requirements: 8_

- [ ] 5.3 SequentialBootstrapperの置換
  - 既存のSequentialBootstrapper参照をGlobalBootstrapperに更新
  - 互換性のあるAPIでの置き換え
  - 既存テストの移行
  - _Requirements: 8_

### Phase 6: L4 AAD Integration (pricer_risk)

- [ ] 6. 陰関数定理によるAAD統合
- [ ] 6.1 (P) ImplicitSolverの実装
  - 陰関数定理による感応度計算: ∂L/∂m = J⁻ᵀ · ∂L/∂x*
  - SolverResultからJacobian逆行列を取得
  - CurveSensitivities結果構造体の実装
  - _Requirements: 6_
  - _Contracts: ImplicitSolver Service_

- [ ] 6.2 (P) Finite differenceフォールバックの実装
  - Jacobian逆行列が利用不可時の数値微分フォールバック
  - 設定可能なイプシロン値
  - 警告ログ出力
  - _Requirements: 6_

### Phase 7: Testing

- [ ] 7. テストスイートの実装
- [ ] 7.1 ユニットテストの実装
  - SystemOfEquationsの数値Jacobianテスト（解析解との比較）
  - MultidimensionalNewtonSolverの収束テスト（線形・二次関数）
  - SolverResultのJacobian逆行列検証（A * A⁻¹ = I）
  - エラーケーステスト（特異Jacobian、次元不整合）
  - _Requirements: 11_

- [ ] 7.2 統合テストの実装
  - CurveCalibrationProblem + MultidimensionalNewtonSolver統合テスト
  - GlobalBootstrapperのend-to-endテスト
  - ImplicitSolver AAD計算テスト（finite differenceとの比較）
  - _Requirements: 11_

- [ ] 7.3 パフォーマンスベンチマークの実装
  - 30商品キャリブレーション時間測定（目標: < 10ms）
  - Jacobian逆行列計算オーバーヘッド測定
  - AAD vs Bump-and-Revalue速度比較
  - criterionベンチマーク設定
  - _Requirements: 10, 11_

### Phase 8: Integration and Verification

- [ ] 8. 最終統合と検証
- [ ] 8.1 エンドツーエンド統合テスト
  - 市場データからカーブ構築→商品価格計算→リスク感応度計算の全フロー検証
  - 既存SequentialBootstrapperとの結果比較（数値的一貫性）
  - エラーハンドリングの全パス確認
  - _Requirements: 10, 11_

- [ ] 8.2 AAD検証テスト
  - ImplicitSolverによる感応度とfinite differenceの比較
  - 許容誤差範囲の確認（1e-6程度）
  - パフォーマンス目標達成の確認（5x speedup）
  - _Requirements: 6, 10, 11_

---

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1. 多次元Newton-Raphson | 2.1, 2.2, 2.3 |
| 2. SystemOfEquations | 1.2, 4.2 |
| 3. SolverResult | 1.3, 2.3 |
| 4. CurveCalibrationProblem | 4.1, 4.2 |
| 5. CalibrationInstrument | 3.1, 3.2 |
| 6. AAD陰関数定理 | 6.1, 6.2, 8.2 |
| 7. 線形代数演算 | 2.2, 2.3 |
| 8. GlobalBootstrapper | 5.1, 5.2, 5.3 |
| 9. エラーハンドリング | 1.1 |
| 10. パフォーマンス | 7.3, 8.1, 8.2 |
| 11. テスト | 7.1, 7.2, 7.3, 8.1, 8.2 |

---

## Parallel Execution Notes

以下のタスクは並列実行可能:
- **1.1, 1.2, 1.3**: 異なるファイル・型定義、依存関係なし
- **6.1, 6.2**: L4層の独立コンポーネント、L1/L2完了後に並列可能

Phase間の依存関係:
- Phase 2 → Phase 1 に依存
- Phase 3 → Phase 1 に依存（Phase 2と並列可能）
- Phase 4 → Phase 1, 3 に依存
- Phase 5 → Phase 2, 4 に依存
- Phase 6 → Phase 1 に依存（Phase 3-5と部分的に並列可能）
- Phase 7, 8 → 全実装完了後
