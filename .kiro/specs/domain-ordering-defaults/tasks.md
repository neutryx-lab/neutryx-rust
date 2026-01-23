# Implementation Plan

## Overview

本タスクリストは、Neutryx ライブラリの enum 型を業務標準の並び順に統一するリファクタリングを実装する。

---

- [x] 1. Frequency enum の並び順変更（infra_master）
- [x] 1.1 (P) Frequency variant 順序を高頻度→低頻度に変更
  - 現在の `Annual → SemiAnnual → Quarterly → Monthly → Weekly → Daily` を逆順に並べ替え
  - `Daily → Weekly → Monthly → Quarterly → SemiAnnual → Annual` の順序で variant を定義
  - `#[default]` 属性は `Monthly` に維持
  - `PartialOrd`, `Ord` 派生により `Daily < Weekly < ... < Annual` の順序を保証
  - _Requirements: 1.1, 1.2, 1.3_

- [x] 1.2 (P) Frequency ドキュメントコメント追加
  - enum レベルに並び順理由を説明するドキュメント追加（"Ordered by frequency: highest to lowest"）
  - 各 variant に年間支払回数を記載したドキュメント追加
  - 新 variant 追加時のガイダンス（順序維持の方法）を記載
  - _Requirements: 1.4, 8.1, 8.2_

---

- [x] 2. Frequency enum 同期（pricer_models）
- [x] 2.1 pricer_models の Frequency を infra_master と同期
  - `pricer_models::market::calibration::bootstrapping::instrument::Frequency` の variant 順序を変更
  - `Annual → SemiAnnual → Quarterly → Monthly → Daily` を `Daily → Monthly → Quarterly → SemiAnnual → Annual` に変更
  - 注意: pricer_models 版には `Weekly` が存在しない
  - _Requirements: 1.1, 1.2, 1.3_

---

- [x] 3. BootstrapInterpolation 順序調整
- [x] 3.1 (P) FlatForward を2番目の位置に移動
  - 現在: `LogLinear → LinearZeroRate → CubicSpline → MonotonicCubic → FlatForward`
  - 変更後: `LogLinear → FlatForward → LinearZeroRate → CubicSpline → MonotonicCubic`
  - `#[default]` は `LogLinear` を維持
  - _Requirements: 4.1, 4.2_

- [x] 3.2 (P) BootstrapInterpolation ドキュメントコメント追加
  - enum レベルに業界使用頻度順の説明を追加
  - 各補間方式の特徴と使用場面を記載
  - _Requirements: 8.1, 8.2_

---

- [x] 4. 既存正順序 enum のドキュメント追加
- [x] 4.1 (P) RateType ドキュメント追加
  - アセットクラス別グループ化の理由を説明
  - 金利商品 → FX → ボラティリティの順序根拠を記載
  - _Requirements: 2.1, 2.2, 2.3, 8.1, 8.2_

- [x] 4.2 (P) StochasticModelEnum ドキュメント追加
  - モデル複雑度順（GBM → Heston → SABR → HW → CIR）の説明を追加
  - 各モデルの複雑度レベル（Level 1-3）を記載
  - feature-flag による条件付きコンパイルの説明を追加
  - _Requirements: 3.1, 3.2, 3.3, 8.1, 8.2_

- [x] 4.3 (P) CurveName ドキュメント追加
  - 論理グループ順（オーバーナイト → インターバンク → 機能別 → カスタム）の説明を追加
  - 各カーブ名の用途を記載
  - _Requirements: 5.1, 5.2, 8.1, 8.2_

- [x] 4.4 (P) 維持対象 enum（Tenor, AssetClass, QuoteType, DayCounter, BDC）のドキュメント確認
  - 既存順序が正しいことを確認し、必要に応じてドキュメント補完
  - Tenor: 期間順、AssetClass: 銀行組織順、QuoteType: 市場慣行順
  - DayCounter: 数学ファミリー別、BusinessDayConvention: 論理順
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 8.1_

---

- [x] 5. Serde 後方互換性確認
- [x] 5.1 Serde シリアライゼーション方式の確認
  - 全対象 enum が name-based serialization を使用していることを確認
  - `#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]` の存在確認
  - variant 名変更がないことを確認
  - _Requirements: 7.1, 7.2, 7.3_

---

- [x] 6. テスト実装
- [x] 6.1 Frequency 順序テスト追加
  - `Ord` trait による比較テスト: `Daily < Weekly < Monthly < Quarterly < SemiAnnual < Annual`
  - `periods_per_year()` の戻り値テスト
  - `Vec<Frequency>` のソートテスト
  - _Requirements: 1.2, 1.3, 1.4_

- [x] 6.2 BootstrapInterpolation Default テスト追加
  - `Default::default()` が `LogLinear` を返すことを確認（既存テスト確認済み）
  - _Requirements: 4.2_

- [x] 6.3 (P) Serde ラウンドトリップテスト追加
  - Frequency, BootstrapInterpolation の serialize → deserialize 往復テスト
  - 順序変更前後で同一 JSON 表現を確認（name-based serialization 確認済み）
  - _Requirements: 7.1, 7.2_

---

- [x] 7. 回帰テスト・検証
- [x] 7.1 ワークスペース全体テスト実行
  - `cargo test -p infra_master` で 207 テスト通過を確認
  - `cargo test -p pricer_models` で関連テスト通過を確認
  - 注: clippy は既存の無関係な警告あり（本変更とは無関係）
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 7.3_

---

## Requirements Coverage Matrix

| Task | Requirements |
|------|--------------|
| 1.1 | 1.1, 1.2, 1.3 |
| 1.2 | 1.4, 8.1, 8.2 |
| 2.1 | 1.1, 1.2, 1.3 |
| 3.1 | 4.1, 4.2 |
| 3.2 | 8.1, 8.2 |
| 4.1 | 2.1, 2.2, 2.3, 8.1, 8.2 |
| 4.2 | 3.1, 3.2, 3.3, 8.1, 8.2 |
| 4.3 | 5.1, 5.2, 8.1, 8.2 |
| 4.4 | 6.1, 6.2, 6.3, 6.4, 6.5, 8.1 |
| 5.1 | 7.1, 7.2, 7.3 |
| 6.1 | 1.2, 1.3, 1.4 |
| 6.2 | 4.2 |
| 6.3 | 7.1, 7.2 |
| 7.1 | 6.1, 6.2, 6.3, 6.4, 6.5, 7.3 |

---

## Implementation Notes

- **並行実行可能タスク**: 1.1, 1.2, 3.1, 3.2, 4.1, 4.2, 4.3, 4.4 はファイル競合なしで並行実行可能
- **依存関係**: タスク 2.1 は 1.1 完了後に実行（順序の一貫性確保）、タスク 6-7 は 1-5 完了後
- **既存コードへの影響**: match パターンは variant 名ベースのため影響なし
- **ロールバック**: variant 順序を元に戻すのみで完全に復元可能
