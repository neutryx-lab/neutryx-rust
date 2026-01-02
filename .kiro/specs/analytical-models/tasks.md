# 実装タスク: analytical-models

## タスク概要

本実装計画は、ヨーロピアンオプションの解析的プライシングモデル（Black-Scholes および Bachelier）を `pricer_models` クレート (L2) に構築する。全8要件を8のメジャータスクと22のサブタスクに分割し、累積正規分布関数、閉形式価格計算、解析的グリークス、VanillaOption 統合を実現する。

**実装フォーカス**:
- 累積正規分布関数 (norm_cdf, norm_pdf)
- Black-Scholes モデル（価格、d1/d2、Greeks）
- Bachelier (正規) モデル
- AnalyticalError エラー型
- VanillaOption 統合インターフェース
- AD 互換性検証

**テスト戦略**:
- ユニットテスト（各関数・メソッド）
- Put-call parity 検証
- 解析的 Greeks vs 有限差分
- property-based テスト

---

## 実装タスク

### Phase 1: 基盤コンポーネント

- [x] 1. エラー型定義
- [x] 1.1 (P) AnalyticalError enum の作成
  - analytical モジュール専用のエラー型を定義
  - InvalidVolatility, InvalidSpot, UnsupportedExerciseStyle, NumericalInstability バリアントを実装
  - thiserror を使用した Display 自動導出
  - PricingError への From トレイト実装
  - エラーメッセージは British English で記述
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_

- [x] 2. 累積正規分布関数
- [x] 2.1 (P) norm_cdf 関数の実装
  - erfc ベースの標準正規 CDF を実装
  - 数式: Φ(x) = 0.5 * erfc(-x / sqrt(2))
  - ジェネリック `T: Float` で f64/Dual64 対応
  - 条件分岐を回避して AD 互換性を確保
  - 極端な入力 (|x| > 8) でも安定した結果を返却
  - _Requirements: 4.1, 4.2, 4.4, 4.5_

- [x] 2.2 (P) norm_pdf 関数の実装
  - 標準正規 PDF を実装
  - 数式: φ(x) = (1 / sqrt(2π)) * exp(-x² / 2)
  - ジェネリック `T: Float` 対応
  - _Requirements: 4.3_

- [x] 2.3 分布関数のユニットテスト
  - norm_cdf の参照値比較テスト（精度 < 1e-7）
  - norm_pdf の参照値比較テスト
  - 対称性テスト: norm_cdf(-x) + norm_cdf(x) ≈ 1
  - 境界値テスト: x = 0, ±3, ±8
  - property-based テスト: 結果が [0, 1] 範囲内
  - _Requirements: 4.2, 4.5_

### Phase 2: Black-Scholes 価格計算

- [x] 3. d1/d2 項計算
- [x] 3.1 BlackScholes 構造体と d1/d2 メソッドの実装
  - BlackScholes<T: Float> 構造体を定義（spot, rate, volatility）
  - コンストラクタで spot > 0, volatility > 0 を検証
  - d1 項計算: (ln(S/K) + (r + σ²/2) * T) / (σ * sqrt(T))
  - d2 項計算: d1 - σ * sqrt(T)
  - sqrt(T) を事前計算して効率化
  - expiry ≈ 0 のリミットケース処理
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

- [x] 4. Black-Scholes 価格計算
- [x] 4.1 price_call メソッドの実装
  - Black-Scholes コール価格公式を実装
  - C = S * N(d1) - K * exp(-r * T) * N(d2)
  - expiry = 0 の場合は intrinsic value を返却
  - _Requirements: 1.1, 1.2, 1.5_

- [x] 4.2 price_put メソッドの実装
  - Black-Scholes プット価格公式を実装
  - P = K * exp(-r * T) * N(-d2) - S * N(-d1)
  - expiry = 0 の場合は intrinsic value を返却
  - _Requirements: 1.1, 1.3, 1.5_

- [x] 4.3 Put-call parity 検証テスト
  - C - P = S - K * exp(-r * T) の検証
  - 複数パラメータセットでテスト
  - 許容誤差内での一致を確認
  - _Requirements: 1.4_

### Phase 3: 解析的グリークス

- [x] 5. グリークス計算
- [x] 5.1 (P) Delta 計算の実装
  - コール Delta = N(d1)
  - プット Delta = N(d1) - 1
  - is_call パラメータでコール/プット判定
  - _Requirements: 3.1_

- [x] 5.2 (P) Gamma 計算の実装
  - Gamma = φ(d1) / (S * σ * sqrt(T))
  - コール/プット共通
  - _Requirements: 3.2_

- [x] 5.3 (P) Vega 計算の実装
  - Vega = S * sqrt(T) * φ(d1)
  - コール/プット共通
  - _Requirements: 3.3_

- [x] 5.4 (P) Theta 計算の実装
  - コール Theta = -(S * σ * φ(d1)) / (2 * sqrt(T)) - r * K * exp(-r * T) * N(d2)
  - プット Theta = -(S * σ * φ(d1)) / (2 * sqrt(T)) + r * K * exp(-r * T) * N(-d2)
  - _Requirements: 3.4_

- [x] 5.5 (P) Rho 計算の実装
  - コール Rho = K * T * exp(-r * T) * N(d2)
  - プット Rho = -K * T * exp(-r * T) * N(-d2)
  - _Requirements: 3.5_

- [x] 5.6 グリークス検証テスト
  - 解析的 Greeks vs 有限差分の比較
  - Delta 範囲: コール [0, 1], プット [-1, 0]
  - Gamma/Vega は非負
  - ATM、ITM、OTM でのテスト
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

### Phase 4: Bachelier モデル

- [x] 6. Bachelier モデル実装
- [x] 6.1 Bachelier 構造体と price_call の実装
  - Bachelier<T: Float> 構造体を定義（forward, volatility）
  - コンストラクタで volatility > 0 を検証（forward は負を許容）
  - 正規モデル価格公式: C = (F - K) * N(d) + σ * sqrt(T) * φ(d)
  - d = (F - K) / (σ * sqrt(T))
  - _Requirements: 2.1, 2.2, 2.4_

- [x] 6.2 Bachelier price_put の実装
  - 正規モデルプット価格公式を実装
  - P = (K - F) * N(-d) + σ * sqrt(T) * φ(d)
  - _Requirements: 2.3_

- [x] 6.3 Bachelier テストと Clone/Debug 導出
  - Clone, Debug derive を確認
  - Put-call parity: C - P = F - K の検証
  - 負のフォワード価格でのテスト
  - _Requirements: 2.4, 2.5_

### Phase 5: 統合インターフェース

- [x] 7. VanillaOption 統合
- [x] 7.1 BlackScholes::price_option の実装
  - VanillaOption を引数に取る統合メソッド
  - strike, expiry, payoff_type を抽出して価格計算
  - notional スケーリングを適用
  - 非 European exercise_style の場合は UnsupportedExerciseStyle エラー
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

- [x] 7.2 Bachelier::price_option の実装
  - VanillaOption を引数に取る統合メソッド
  - BlackScholes と同様のロジック
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

- [x] 7.3 統合テスト
  - price_option メソッドのエンドツーエンドテスト
  - American オプションでのエラー返却テスト
  - AnalyticalError → PricingError 変換テスト
  - _Requirements: 6.4, 7.5_

### Phase 6: AD 互換性と最終検証

- [x] 8. ジェネリック型互換性検証
- [x] 8.1 f64 での全機能テスト
  - BlackScholes<f64> での価格・Greeks 計算
  - Bachelier<f64> での価格計算
  - 全メソッドの動作確認
  - _Requirements: 5.1_

- [x] 8.2* Dual64 での AD 互換性テスト
  - num-dual クレートの Dual64 型でインスタンス化
  - 価格計算を通じて導関数が伝播することを確認
  - AD 計算による Delta vs 解析的 Delta の比較
  - pricer_core::math::smoothing 関数の使用確認
  - AD テープ一貫性の検証
  - _Requirements: 5.2, 5.3, 5.4, 5.5_

- [x] 8.3 モジュールエクスポートと統合確認
  - analytical/mod.rs でのパブリックエクスポート設定
  - lib.rs での analytical モジュール公開
  - cargo test --package pricer_models の全テスト通過
  - cargo doc で警告なし
  - _Requirements: 5.1_

---

## タスクサマリー

- **メジャータスク**: 8
- **サブタスク**: 22
- **並列実行可能タスク**: 9 (P マーク)
- **オプションテスト**: 1 (* マーク)
- **要件カバレッジ**: 全8要件 (40 acceptance criteria)

### 要件マッピング検証

| 要件 | カバーするタスク |
|------|----------------|
| Requirement 1 (1.1-1.5) | 4.1, 4.2, 4.3, 3.1 |
| Requirement 2 (2.1-2.5) | 6.1, 6.2, 6.3 |
| Requirement 3 (3.1-3.5) | 5.1, 5.2, 5.3, 5.4, 5.5, 5.6 |
| Requirement 4 (4.1-4.5) | 2.1, 2.2, 2.3 |
| Requirement 5 (5.1-5.5) | 8.1, 8.2, 8.3 |
| Requirement 6 (6.1-6.5) | 7.1, 7.2, 7.3 |
| Requirement 7 (7.1-7.5) | 1.1, 7.3 |
| Requirement 8 (8.1-8.5) | 3.1 |

### 依存関係フロー

```
Phase 1 (基盤 - 並列可能)
  1.1 (P) AnalyticalError
  2.1 (P) norm_cdf
  2.2 (P) norm_pdf
  2.3 分布テスト (depends on 2.1, 2.2)
  ↓
Phase 2 (Black-Scholes 価格)
  3.1 BlackScholes + d1/d2 (depends on 2.1)
  4.1 price_call (depends on 3.1)
  4.2 price_put (depends on 3.1)
  4.3 Put-call parity (depends on 4.1, 4.2)
  ↓
Phase 3 (Greeks - 並列可能)
  5.1-5.5 (P) Delta, Gamma, Vega, Theta, Rho (depends on 3.1, 2.2)
  5.6 Greeks テスト (depends on 5.1-5.5)
  ↓
Phase 4 (Bachelier - 並列可能)
  6.1, 6.2, 6.3 (depends on 2.1, 2.2, 1.1)
  ↓
Phase 5 (統合)
  7.1 BS price_option (depends on 4.1, 4.2)
  7.2 BA price_option (depends on 6.1, 6.2)
  7.3 統合テスト (depends on 7.1, 7.2)
  ↓
Phase 6 (検証)
  8.1 f64 テスト (depends on all)
  8.2* Dual64 テスト (depends on all)
  8.3 モジュール統合 (depends on all)
```

---

**生成日時**: 2026-01-01
**言語**: 日本語
**対象**: pricer_models Layer 2 解析的プライシングモジュール
