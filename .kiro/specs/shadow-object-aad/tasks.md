# Implementation Plan

## Task Summary

- **合計**: 5 メジャータスク、16 サブタスク
- **要件カバレッジ**: 全8要件（1.1-1.5, 2.1-2.6, 3.1-3.7, 4.1-4.5, 5.1-5.5, 6.1-6.5, 7.1-7.5, 8.1-8.5）
- **推定作業時間**: 各サブタスク 1-3 時間

---

## Tasks

- [x] 1. Shadow Trait 基盤構築
- [x] 1.1 Shadow トレイト定義と基本型実装
  - `Shadow` トレイトを `pricer_risk::enzyme::shadow` モジュールに定義
  - `Clone` bound を持つトレイトとして `zero_out()` と `create_shadow()` メソッドを提供
  - `f64` および `Vec<f64>` に対する `Shadow` 実装を追加
  - `zero_out()` は全数値フィールドを `0.0` にリセット
  - `create_shadow()` はデフォルト実装として `clone()` + `zero_out()` を提供
  - _Requirements: 1.1, 1.2, 1.3, 1.4_

- [x] 1.2 (P) マーケットデータ構造への Shadow 実装
  - `YieldCurve` 構造体に対する `Shadow` トレイト実装を追加
  - `VolSurface` 構造体に対する `Shadow` トレイト実装を追加
  - ネスト構造の `zero_out()` 呼び出しを再帰的に実装
  - 勾配オブジェクトが元の構造体と同一のメモリレイアウトを持つことを保証
  - 同一型構造によりデバッグ時の直感的なアクセスを実現（`d_market.rates[i]`）
  - _Requirements: 1.5, 6.1, 6.2, 6.3, 6.4, 6.5_

- [x] 1.3 (P) Shadow トレイト単体テスト
  - `zero_out()` が全フィールドを `0.0` にすることを検証
  - `create_shadow()` が元のオブジェクトを変更しないことを検証
  - ネスト構造での再帰的ゼロ初期化を検証
  - 複数カーブを含む構造体でのカーブ identity 保持を検証
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 6.5_

---

- [x] 2. スライスベース・カーネル実装
- [x] 2.1 プライシングカーネル関数定義
  - `pricer_risk::enzyme::kernel` モジュールにカーネル関数を定義
  - Active inputs（微分対象）用の `&[f64]` スライス引数を設計
  - Const inputs（定数）用の `&[f64]` スライス引数を設計
  - 出力は `&mut f64` パラメータへの書き込みで実現
  - 関数内でのヒープアロケーションを禁止（hot path 最適化）
  - 具象 `f64` 型のみを使用（ジェネリクス・Dual numbers 不使用）
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.6, 5.1, 5.2, 5.3_

- [x] 2.2 Enzyme autodiff マクロ適用
  - `#[autodiff]` マクロを使用してカーネル関数を微分可能に
  - `Duplicated` フラグで Active inputs を指定
  - `Const` フラグで定数 inputs を指定
  - `enzyme-ad` feature flag でのコンパイル分離を確保
  - Feature なしビルド時の fallback 実装を提供
  - _Requirements: 2.5, 8.2, 8.4_

- [x] 2.3 (P) カーネル関数単体テスト
  - `pricing_kernel` が正しい PV を計算することを検証（解析解比較）
  - `d_pricing_kernel`（Enzyme 自動生成）が正しい勾配を計算することを検証
  - Finite difference との比較による勾配精度検証
  - 空スライス入力に対するエラーハンドリング検証
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6_

---

- [ ] 3. AAD バインダー層構築
- [ ] 3.1 RiskResult 型と ActivityMask 定義
  - `RiskResult<M: Shadow>` 構造体を定義（`pv: f64`, `gradients: M`）
  - `ActivityMask` 構造体を定義（`rates_active`, `volatilities_active`, `fx_rates_active`）
  - `ActivityMask::default()` で全コンポーネントを active に設定
  - const 指定されたコンポーネントの shadow 値が 0.0 のままであることを保証
  - _Requirements: 7.1, 7.5_

- [ ] 3.2 MarketRiskCalculator トレイト実装
  - `MarketRiskCalculator<M: Shadow, T>` トレイトを定義
  - `calculate_risk(market, trade, mask)` メソッドを設計
  - マーケットデータ構造から `&[f64]` スライスを抽出するロジックを実装
  - Shadow オブジェクトから `&mut [f64]` スライスを抽出して勾配バッファとして使用
  - `ENZYME_DUP`/`ENZYME_CONST` フラグによる部分微分制御を実装
  - 計算完了後に `(pv, shadow)` を `RiskResult` として返却
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7_

- [ ] 3.3 ゼロコピー・データ受け渡し実装
  - `as_ptr()`, `as_mut_ptr()` を使用したポインタ渡しを実装
  - `&self.rates[..]` 構文によるゼロコピースライス抽出
  - 中間 Pack/Unpack なしでカーネルを呼び出し可能に
  - カーネル実行中のソースデータの有効性を保証
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

- [ ] 3.4 (P) バインダー層単体テスト
  - `calculate_risk` が正しい PV と勾配を返すことを検証
  - `ActivityMask` による部分微分の動作を検証（rates のみ、vols のみ）
  - const コンポーネントの勾配が 0.0 であることを検証
  - 複数 Active コンポーネントの組み合わせを検証
  - _Requirements: 7.2, 7.3, 7.4_

---

- [ ] 4. 既存 pricer_risk 統合
- [ ] 4.1 enzyme モジュールへの統合
  - `shadow.rs`, `kernel.rs`, `binder.rs` を `pricer_risk::enzyme` に配置
  - 既存の `mod.rs`（`ADMode`, `Activity`）との整合性を確保
  - `enzyme-ad` feature flag との互換性を維持
  - A-I-P-S 依存ルール（L4 は L1-L3 に依存、S/A には依存しない）を遵守
  - _Requirements: 8.1, 8.2, 8.5_

- [ ] 4.2 GreeksEnzyme トレイトとの連携
  - 既存 `GreeksEnzyme` トレイトインフラとの統合パスを確立
  - `EnzymeGreeksResult` との型互換性を確保
  - 既存の non-AAD コードパスとの後方互換性を維持
  - _Requirements: 8.3, 5.4, 5.5_

- [ ] 4.3 (P) エラー型定義
  - `ShadowAadError` 列挙型を定義（`LengthMismatch`, `EmptySlice`, `EnzymeNotAvailable`）
  - 入力検証での早期エラー検出を実装
  - `EnzymeNotAvailable` 時の finite difference fallback を実装
  - _Requirements: 8.4_

---

- [ ] 5. 統合テストとパフォーマンス検証
- [ ] 5.1 YieldCurve 統合テスト
  - `YieldCurve` に対する Delta/DV01 計算を bump-and-revalue と比較
  - 1000 要素スライスでの AAD パフォーマンスを測定（target: < 1ms）
  - `clone()` + `zero_out()` のオーバーヘッドを測定
  - _Requirements: 1.1, 1.2, 1.3, 4.1, 4.2_

- [ ] 5.2 (P) VolSurface 統合テスト
  - `VolSurface` に対する Vega 計算を実装・検証
  - 2D 構造（strikes × expiries）での勾配マッピングを検証
  - _Requirements: 6.1, 6.4, 7.3_

- [ ] 5.3 (P) Feature flag 検証テスト
  - `enzyme-ad` feature 有効/無効での結果一致を確認
  - Feature なしビルドで Enzyme 依存なしでコンパイルできることを検証
  - _Requirements: 8.2, 8.4_

---

## Requirements Coverage Matrix

| Requirement | Tasks |
|-------------|-------|
| 1.1-1.5 | 1.1, 1.2, 1.3 |
| 2.1-2.6 | 2.1, 2.2, 2.3 |
| 3.1-3.7 | 3.2, 3.3 |
| 4.1-4.5 | 3.3, 5.1 |
| 5.1-5.5 | 2.1, 4.2 |
| 6.1-6.5 | 1.2, 5.2 |
| 7.1-7.5 | 3.1, 3.4 |
| 8.1-8.5 | 2.2, 4.1, 4.2, 4.3, 5.3 |

## Parallel Execution Notes

以下のタスクは並列実行可能（`(P)` マーカー付き）:
- **1.2, 1.3**: Shadow 実装とテストは独立して作業可能
- **2.3**: カーネルテストは実装完了後に並列実行可能
- **3.4**: バインダーテストは他のテストと並列実行可能
- **4.3**: エラー型定義は他の統合作業と並列実行可能
- **5.2, 5.3**: 統合テストは相互に独立

## Implementation Sequence

推奨される実装順序:
1. **Phase 1** (1.1 → 1.2, 1.3): Shadow Trait 基盤
2. **Phase 2** (2.1 → 2.2 → 2.3): カーネル実装
3. **Phase 3** (3.1 → 3.2 → 3.3, 3.4): バインダー層
4. **Phase 4** (4.1 → 4.2, 4.3): 統合作業
5. **Phase 5** (5.1, 5.2, 5.3): 検証・パフォーマンステスト
