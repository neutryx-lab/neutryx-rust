# Implementation Plan

## Task Overview

derive_more クレートを導入し、New Type パターンにおけるボイラープレートコードを削減する。

---

## Phase 1: 依存追加 + Counterparty ID型移行

- [x] 1. ワークスペース依存関係の設定
- [x] 1.1 Cargo.toml (workspace) に derive_more を追加
  - `[workspace.dependencies]` に derive_more v2 を追加
  - features: `from`, `display`, `as_ref`, `add`, `mul` を指定
  - 全クレートで統一バージョンを使用可能にする
  - _Requirements: 1.1, 1.2, 1.3_

- [x] 1.2 (P) infra_domain クレートに derive_more 依存を追加
  - `crates/infra_domain/Cargo.toml` に `derive_more = { workspace = true }` を追加
  - 既存の serde 依存パターンに合わせる
  - _Requirements: 1.3_

- [x] 2. Counterparty ID 型の移行
- [x] 2.1 CounterPartyId を derive_more に移行
  - 手動 Display 実装を `#[derive(Display)]` に置換
  - 手動 From 実装を `#[derive(From)]` に置換
  - `new()`, `as_str()` メソッドは維持
  - 既存テストが引き続きパスすることを確認
  - _Requirements: 3.1, 3.3, 4.1, 4.2, 5.2, 5.3_

- [x] 2.2 (P) NettingSetId を derive_more に移行
  - Display, From derive を適用
  - 既存の動作を維持
  - _Requirements: 3.1, 4.1, 5.2, 5.3_

- [x] 2.3 (P) CcpId を derive_more に移行
  - Display, From derive を適用
  - 既存の動作を維持
  - _Requirements: 3.1, 4.1, 5.2, 5.3_

- [x] 2.4 (P) IsdaAgreementId を derive_more に移行
  - Display, From derive を適用
  - 既存の動作を維持
  - _Requirements: 3.1, 4.1, 5.2, 5.3_

- [x] 2.5 (P) VariationMarginAgreementId を derive_more に移行
  - Display, From derive を適用
  - 既存の動作を維持
  - _Requirements: 3.1, 4.1, 5.2, 5.3_

- [x] 2.6 (P) CrossBookNettingAgreementId を derive_more に移行
  - Display, From derive を適用
  - 既存の動作を維持
  - _Requirements: 3.1, 4.1, 5.2, 5.3_

- [x] 3. Phase 1 検証
- [x] 3.1 ビルド検証と AD 互換性確認
  - `cargo build -p infra_domain --features serde` でビルド確認
  - `cargo test -p infra_domain` でテスト確認
  - `cargo build -p pricer_risk --features enzyme-ad` で AD 互換性確認
  - `Box<dyn Trait>` パターンが導入されていないことを確認
  - _Requirements: 6.1, 6.2, 6.3_

---

## Phase 2: define_id! マクロ簡略化

- [x] 4. define_id! マクロの更新
- [x] 4.1 define_id! マクロを derive_more 対応に更新
  - マクロ本体で Display, From derive を追加
  - `new()`, `as_str()`, `AsRef<str>` 実装は維持
  - 手動 Display/From 実装を削除
  - TradeId, PortfolioId, BookId が正しく生成されることを確認
  - _Requirements: 3.1, 3.2, 4.1, 5.1, 5.2_

- [x] 4.2 マクロ生成型のテスト追加
  - TradeId, PortfolioId, BookId の Display 動作テスト
  - From<String>, From<&str> 変換テスト
  - 既存テストとの互換性確認
  - _Requirements: 7.3_

---

## Phase 3: 数値型移行 + テスト + ドキュメント

- [x] 5. 数値型 NewType の移行
- [x] 5.1 BasisSpread に算術演算 derive を適用
  - Add, Sub を derive（Display はカスタム形式を維持）
  - `from_bps()`, `bps()`, `as_decimal()` メソッドは維持
  - NewType ラッパーセマンティクスが保持されることを確認
  - _Requirements: 2.1, 2.2, 3.1, 3.2, 4.1_

- [x] 6. テストカバレッジ
- [x] 6.1 ID 型の Display/From 単体テスト追加
  - CounterPartyId, NettingSetId 等の Display テスト
  - From<String>, From<&str> 変換テスト
  - 既存テストとの整合性確認
  - _Requirements: 7.1, 7.3_

- [x] 6.2 (P) 算術演算の property-based テスト追加
  - proptest による可換性テスト (a + b == b + a)
  - proptest による結合性テスト ((a + b) + c == a + (b + c))
  - f64 精度制限を考慮した approx 比較
  - _Requirements: 7.2_

- [x] 7. ドキュメンテーション
- [x] 7.1 Steering ドキュメントに NewType ガイドライン追加
  - `.kiro/steering/ai_rules.md` に derive_more 使用パターンを追記
  - ID 型（String ラッパー）の推奨 derive を記載
  - 数値型（f64 ラッパー）の推奨 derive を記載
  - derive_more を使用しない場合（バリデーションロジック、カスタム演算）の説明を追加
  - _Requirements: 8.1, 8.2, 8.3_

---

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1.1, 1.2, 1.3 | 1.1, 1.2 |
| 2.1, 2.2, 2.3, 2.4, 2.5 | 5.1 |
| 3.1, 3.2, 3.3, 3.4 | 2.1-2.6, 4.1, 5.1 |
| 4.1, 4.2, 4.3 | 2.1-2.6, 4.1, 5.1 |
| 5.1, 5.2, 5.3, 5.4 | 2.1-2.6, 4.1 |
| 6.1, 6.2, 6.3 | 3.1 |
| 7.1, 7.2, 7.3 | 4.2, 6.1, 6.2 |
| 8.1, 8.2, 8.3 | 7.1 |
