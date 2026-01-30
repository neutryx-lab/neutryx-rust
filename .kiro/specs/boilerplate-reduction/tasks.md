# Implementation Plan: boilerplate-reduction

## Overview

本タスクリストは `bon` クレートによる Builder パターン自動生成への段階的移行を実装するためのものです。Phase 0 から Phase 4 まで、単純な構造体から複雑な構造体へと段階的に移行を進めます。

---

## Tasks

### Phase 0: 依存関係セットアップ

- [ ] 1. bon クレートをワークスペースに追加
- [ ] 1.1 ワークスペース Cargo.toml に bon 依存関係を追加
  - `[workspace.dependencies]` セクションに `bon = "3.6"` を追加
  - infra_master クレートの Cargo.toml で `bon = { workspace = true }` を追加
  - `cargo build --workspace` でビルド成功を確認
  - _Requirements: 1.1, 1.2, 1.3_

- [ ] 1.2 dependency-management.md ステアリングを更新
  - bon クレートをワークスペース依存関係として文書化
  - bon の用途（Builder 自動生成）を記載
  - _Requirements: 5.3_

---

### Phase 1: 単純な Builder の移行

- [ ] 2. Book 構造体に bon Builder を適用
- [ ] 2.1 (P) Book 構造体を bon Builder に移行
  - `#[derive(bon::Builder)]` を Book 構造体に追加
  - book_id, name フィールドに `#[builder(into)]` を適用
  - オプショナルフィールドに `#[builder(default)]` を適用
  - 既存の BookBuilder 手書き実装を削除
  - _Requirements: 3.1, 3.2_

- [ ] 2.2 (P) Book Builder の動作確認テスト
  - `Book::builder().book_id("ID").name("Name").build()` パターンが動作することを確認
  - デフォルト値が正しく設定されることを確認
  - 既存の Book 関連テストが全てパスすることを確認
  - _Requirements: 4.1, 4.3_

- [ ] 3. Portfolio 構造体に bon Builder を適用
- [ ] 3.1 (P) Portfolio 構造体を bon Builder に移行
  - `#[derive(bon::Builder)]` を Portfolio 構造体に追加
  - Book と同様のパターンで属性を適用
  - 既存の PortfolioBuilder 手書き実装を削除
  - _Requirements: 3.1, 3.2_

- [ ] 3.2 (P) Portfolio Builder の動作確認テスト
  - Portfolio Builder の基本動作を確認
  - 既存の Portfolio 関連テストが全てパスすることを確認
  - _Requirements: 4.1, 4.3_

---

### Phase 2: デフォルト値を持つ Builder の移行

- [ ] 4. CounterParty 構造体に bon Builder を適用
- [ ] 4.1 CounterParty 構造体を bon Builder に移行
  - `#[derive(bon::Builder)]` を CounterParty 構造体に追加
  - id, name に `#[builder(into)]` を適用
  - sector のデフォルト値を `#[builder(default)]` で設定
  - オプショナルフィールド（lei, country, rating, credit_params）に適切な属性を設定
  - 既存の CounterPartyBuilder 手書き実装を削除
  - Phase 1 完了後に実行（Book/Portfolio の変更がマージ済みであること）
  - _Requirements: 3.1, 3.2, 3.4_

- [ ] 4.2 CounterParty Builder の動作確認テスト
  - CounterParty Builder の基本動作を確認
  - 既存の CounterParty 関連テストが全てパスすることを確認
  - _Requirements: 4.1, 4.3_

- [ ] 5. CsaTerms 構造体に bon Builder を適用
- [ ] 5.1 CsaTerms 構造体を bon Builder に移行
  - `#[derive(bon::Builder)]` を CsaTerms 構造体に追加
  - mpor_days に `#[builder(default = 10)]` を適用
  - margin_currency に `#[builder(default = Currency::USD)]` を適用
  - 既存のデフォルト値ロジック（empty vec → default など）を属性で再現
  - 既存の CsaTermsBuilder 手書き実装を削除
  - _Requirements: 3.1, 3.2, 3.4_

- [ ] 5.2 CsaTerms Builder の動作確認テスト
  - CsaTerms Builder の基本動作を確認
  - デフォルト値（mpor_days=10, margin_currency=USD）が正しく設定されることを確認
  - 既存の CsaTerms 関連テストが全てパスすることを確認
  - _Requirements: 4.1, 4.3_

---

### Phase 3: Trade 構造体の移行

- [ ] 6. Trade 構造体に bon Builder を適用
- [ ] 6.1 Trade 構造体を bon Builder に移行
  - `#[derive(bon::Builder)]` を Trade 構造体に追加
  - id フィールドに `#[builder(into)]` を適用
  - legs, trade_type, metadata に適切なデフォルト値を設定
  - 既存の TradeBuilder 手書き実装を削除
  - Phase 2 完了後に実行
  - _Requirements: 2.1, 3.1, 3.2_

- [ ] 6.2 Trade Builder の動作確認テスト
  - Trade Builder の基本動作を確認
  - 複数の Leg を持つ Trade が正しく構築されることを確認
  - 既存の Trade 関連テストが全てパスすることを確認
  - _Requirements: 4.1, 4.3, 4.4_

---

### Phase 4: LegBuilder の設計変更と移行

- [ ] 7. LegConfig 構造体の新規作成
- [ ] 7.1 LegConfig 構造体を設計・実装
  - 新規 LegConfig 構造体を作成し `#[derive(bon::Builder)]` を適用
  - schedule, notional, currency を必須フィールドとして定義
  - direction, day_count に `#[builder(default)]` を適用
  - validate() メソッドを実装（schedule.len() >= 2, notional >= 0.0 のチェック）
  - Phase 3 完了後に実行
  - _Requirements: 2.2, 3.1, 3.2, 3.4_

- [ ] 7.2 LegConfig の変換メソッドを実装
  - into_fixed_leg(rate: f64) -> Result<Leg, TradeError> メソッドを実装
  - into_floating_leg(index: RateIndex, spread: f64) -> Result<Leg, TradeError> メソッドを実装
  - 両メソッド内で validate() を呼び出し、エラー時は早期リターン
  - 既存の LegBuilder::build_fixed/build_floating と同等の Leg 生成ロジックを実装
  - _Requirements: 2.2, 2.4_

- [ ] 7.3 LegConfig の単体テスト作成
  - LegConfig Builder の基本動作テスト
  - validate() が不正入力を適切に拒否することを確認
  - into_fixed_leg/into_floating_leg が正しい Leg を生成することを確認
  - _Requirements: 4.1_

- [ ] 8. 呼び出し側のコード更新
- [ ] 8.1 infra_master 内の LegBuilder 呼び出しを更新
  - infra_master クレート内の LegBuilder 使用箇所を LegConfig に移行
  - `LegBuilder::new(...).build_fixed(rate)` → `LegConfig::builder()...build().into_fixed_leg(rate)?`
  - 移行後、既存の LegBuilder 実装を削除
  - _Requirements: 4.2, 2.5_

- [ ] 8.2 他クレートの LegBuilder 呼び出しを更新
  - pricer_* クレート内の LegBuilder 使用箇所を調査・更新
  - 必要に応じてテストコードの更新
  - `cargo build --workspace` でビルド成功を確認
  - _Requirements: 4.2, 4.4_

- [ ] 8.3 LegBuilder 関連テストの更新と実行
  - 既存の LegBuilder テストを LegConfig パターンに更新
  - 新 API でのテストケース追加
  - `cargo test -p infra_master` で全テストがパスすることを確認
  - _Requirements: 4.1, 4.3_

---

### Phase 5: ドキュメント整備と最終確認

- [ ] 9. ドキュメント整備と最終検証
- [ ] 9.1 (P) 移行した構造体に doc コメントを追加
  - bon Builder の使用パターンを説明する doc コメントを各構造体に追加
  - bon 属性の意図をインラインコメントで説明（必要に応じて）
  - _Requirements: 5.1, 5.2_

- [ ] 9.2 最終リグレッションテスト
  - `cargo test --workspace` で全テストがパスすることを確認
  - `cargo clippy --workspace` で警告がないことを確認
  - bon 導入前後でランタイム動作が変わっていないことを確認
  - _Requirements: 4.1, 4.3, 4.4_

---

## Requirements Coverage

| Requirement | Task(s) | Status |
|-------------|---------|--------|
| 1.1 | 1.1 | ✅ |
| 1.2 | 1.1 | ✅ |
| 1.3 | 1.1 | ✅ |
| 2.1 | 6.1 | ✅ |
| 2.2 | 7.1, 7.2 | ✅ |
| 2.3 | - | Out of Scope |
| 2.4 | 7.2 | ✅ |
| 2.5 | 8.1 | ✅ |
| 3.1 | 2.1, 3.1, 4.1, 5.1, 6.1, 7.1 | ✅ |
| 3.2 | 2.1, 3.1, 4.1, 5.1, 6.1, 7.1 | ✅ |
| 3.3 | - | 対象なし（skip 属性は使用せず） |
| 3.4 | 4.1, 5.1, 7.1 | ✅ |
| 4.1 | 2.2, 3.2, 4.2, 5.2, 6.2, 7.3, 8.3, 9.2 | ✅ |
| 4.2 | 8.1, 8.2 | ✅ |
| 4.3 | 2.2, 3.2, 4.2, 5.2, 6.2, 8.3, 9.2 | ✅ |
| 4.4 | 6.2, 8.2, 9.2 | ✅ |
| 5.1 | 9.1 | ✅ |
| 5.2 | 9.1 | ✅ |
| 5.3 | 1.2 | ✅ |
| 6.1 | - | 完了（research.md） |
| 6.2 | - | 完了（research.md） |
| 6.3 | All phases | ✅ |

---

## Notes

- **Phase 5 (counterparty_portfolio.rs)**: 設計レビューにより、本フェーズの対象外としました。複雑な Builder（8個）は将来の拡張として残します。
- **並列実行**: Phase 1 の Book/Portfolio は並列実行可能（異なるファイルを操作）。Phase 2 以降は依存関係があるため順次実行。
- **validate() の呼び出し**: into_*_leg() 内で validate() を呼び出し、Result を返す設計を採用。
