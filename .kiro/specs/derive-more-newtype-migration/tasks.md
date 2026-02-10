# Implementation Plan

## Task Overview

derive_more クレートを導入し、New Type パターンにおけるボイラープレートコードを削減する。

---

## Phase 1: 依存追加 + Counterparty ID型移行

- [x] 1. ワークスペース依存関係の設定
- [x] 1.1 Cargo.toml (workspace) に derive_more を追加
  - Requirements: 1.1, 1.2, 1.3

- [x] 1.2 infra_domain クレートに derive_more 依存を追加
  - Requirements: 1.3

- [x] 2. Counterparty ID 型の移行
- [x] 2.1 CounterPartyId を derive_more に移行
  - Requirements: 3.1, 3.3, 4.1, 4.2, 5.2, 5.3

- [x] 2.2 NettingSetId を derive_more に移行
  - Requirements: 3.1, 4.1, 5.2, 5.3

- [x] 2.3 CcpId を derive_more に移行
  - Requirements: 3.1, 4.1, 5.2, 5.3

- [x] 2.4 IsdaAgreementId を derive_more に移行
  - Requirements: 3.1, 4.1, 5.2, 5.3

- [x] 2.5 VariationMarginAgreementId を derive_more に移行
  - Requirements: 3.1, 4.1, 5.2, 5.3

- [x] 2.6 CrossBookNettingAgreementId を derive_more に移行
  - Requirements: 3.1, 4.1, 5.2, 5.3

- [x] 3. Phase 1 検証
- [x] 3.1 ビルド検証と AD 互換性確認
  - Requirements: 6.1, 6.2, 6.3

---

## Phase 2: define_id! マクロ簡略化

- [x] 4. define_id! マクロの更新
- [x] 4.1 define_id! マクロを derive_more 対応に更新
  - Requirements: 3.1, 3.2, 4.1, 5.1, 5.2

- [x] 4.2 マクロ生成型のテスト追加
  - Requirements: 7.3

---

## Phase 3: 数値型移行 + テスト + ドキュメント

- [x] 5. 数値型 NewType の移行
- [x] 5.1 BasisSpread に算術演算 derive を適用
  - Requirements: 2.1, 2.2, 3.1, 3.2, 4.1

- [x] 6. テストカバレッジ
- [x] 6.1 ID 型の Display/From 単体テスト追加
  - Requirements: 7.1, 7.3

- [x] 6.2 算術演算の property-based テスト追加
  - Requirements: 7.2

- [x] 7. ドキュメンテーション
- [x] 7.1 Steering ドキュメントに NewType ガイドライン追加
  - Requirements: 8.1, 8.2, 8.3

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
