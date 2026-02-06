# Implementation Tasks: Jump-Aware Curve Definition

## Task Overview

本タスクリストは design.md に基づき、A-I-P-S アーキテクチャの依存方向に従って実装を進める。
各タスクは TDD 手法（テスト先行 → 実装 → リファクタリング）で進める。

**特記事項**: Date→Time 変換メカニズムは Task 3 で明示的に扱う（デザインレビュー Issue #1 対応）。

---

## Phase 1: infra_master 層（定義）

### Task 1: Limit enum の追加（pricer_core）

**Objective**: 左極限・右極限指定用の Limit enum を pricer_core::types に追加する。

**Requirements**: 2.1

**Files**:
- `crates/pricer_core/src/types/mod.rs` — Limit enum 追加およびエクスポート

**Subtasks**:
1. [ ] `Limit` enum を定義（Left, Right, Continuous variants）
2. [ ] `Default` trait 実装（Continuous をデフォルト）
3. [ ] `Copy`, `Clone`, `Debug`, `PartialEq`, `Eq` derive
4. [ ] serde feature gate 付き Serialize/Deserialize
5. [ ] 単体テスト（各 variant の等価性確認）

**Acceptance Criteria**:
- `Limit::default()` が `Limit::Continuous` を返す
- serde 有効時に JSON シリアライズ可能

---

### Task 2: JumpPillar 構造体の追加

**Objective**: JumpPillar 構造体を infra_master に追加し、ジャンプ定義を表現する。

**Requirements**: 1.3, 1.4, 1.5

**Files**:
- `crates/infra_master/src/market/definition/jump_pillar.rs` — 新規作成
- `crates/infra_master/src/market/definition/mod.rs` — エクスポート追加

**Subtasks**:
1. [ ] `JumpPillar` 構造体定義（jump_date, expected_jump_bps, event_reference, confidence）
2. [ ] `JumpPillar::new()` コンストラクタ実装
3. [ ] `with_event_reference()` Builder メソッド
4. [ ] `from_event_instrument()` 変換コンストラクタ
5. [ ] アクセサメソッド（jump_date, expected_jump_bps, confidence, weighted_jump_bps）
6. [ ] serde feature gate 付き Serialize/Deserialize（camelCase）
7. [ ] 単体テスト
   - new() でフィールド初期化確認
   - from_event_instrument() 変換ロジック
   - weighted_jump_bps() = expected_jump_bps * confidence

**Acceptance Criteria**:
- EventInstrument から JumpPillar への変換が正しく動作
- JSON シリアライズで camelCase フィールド名

---

### Task 3: JumpPillar の Date→Time 変換ユーティリティ

**Objective**: JumpPillar の Date を pricer_models で使用する Time（f64 年分数）に変換する機構を実装する。

**Requirements**: 3.2（ブートストラップでの適用）

**Files**:
- `crates/pricer_models/src/market.rs` または新規 `crates/pricer_models/src/utils/date_conversion.rs`

**Design Review Issue #1 対応**:
デザインレビューで指摘された「Date→Time 変換メカニズム」を本タスクで詳細化。

**Subtasks**:
1. [ ] 既存の Date→Time 変換パターンを調査（DayCounter 使用箇所）
2. [ ] `JumpPillar` から `(time: f64, cumulative_offset: f64)` への変換関数を設計
3. [ ] 変換関数実装
   - 入力: `&[JumpPillar]`, valuation_date: Date, day_counter: &DayCounter
   - 出力: `Vec<(f64, f64)>` — (time, cumulative_jump_offset)
4. [ ] 累積オフセット計算ロジック（bps to log-space offset）
5. [ ] 単体テスト
   - 空リスト → 空 Vec
   - 単一 JumpPillar → 正しい time 変換
   - 複数 JumpPillar → 累積計算

**Acceptance Criteria**:
- DayCounter を使用した正確な年分数計算
- ジャンプオフセットが log(discount_factor) 空間で適用可能

---

### Task 4: JumpPillarBuilder の実装

**Objective**: EventInstrument リストから JumpPillar リストを生成する Builder を実装する。

**Requirements**: 4.1, 4.2, 4.3, 4.4, 4.5

**Files**:
- `crates/infra_master/src/market/definition/jump_pillar.rs` — Builder 追加

**Subtasks**:
1. [ ] `JumpPillarBuilder` 構造体定義
2. [ ] `new(events: Vec<EventInstrument>)` コンストラクタ
3. [ ] `with_rate_index(RateIndex)` フィルタ
4. [ ] `with_date_range(start, end)` フィルタ
5. [ ] `with_min_confidence(threshold)` フィルタ
6. [ ] `build() -> Vec<JumpPillar>` — フィルタ適用＋日付ソート
7. [ ] 単体テスト
   - 空入力 → 空出力
   - rate_index フィルタ動作
   - date_range フィルタ動作
   - min_confidence フィルタ動作
   - 結果が jump_date 昇順ソート

**Acceptance Criteria**:
- 全フィルタが正しく連鎖適用
- 出力は常に日付昇順

---

### Task 5: CurveDefinition の拡張

**Objective**: CurveDefinition に jump_pillars フィールドを追加し、バリデーションを拡張する。

**Requirements**: 1.1, 1.2, 6.1-6.6, 7.1-7.5

**Files**:
- `crates/infra_master/src/market/definition/curve.rs` — フィールド追加
- `crates/infra_master/src/market/definition/error.rs` — エラー variant 追加（必要に応じて）

**Subtasks**:
1. [ ] `jump_pillars: Vec<JumpPillar>` フィールド追加
2. [ ] serde: `#[serde(default, skip_serializing_if = "Vec::is_empty")]`
3. [ ] `with_jump_pillars()`, `with_jump_pillar()` Builder メソッド
4. [ ] `jump_pillar_count()`, `has_jumps()` ヘルパー
5. [ ] バリデーション追加
   - [ ] JumpPillar 日付の一意性チェック
   - [ ] confidence 範囲チェック [0.0, 1.0]
   - [ ] 負の discount factor を招くジャンプ検出
   - [ ] 日付範囲警告（instrument pillar との重複）
6. [ ] `CurveDefError` 新規 variant 追加
   - `DuplicateJumpDate(Date)`
   - `InvalidConfidence { date: Date, value: f64 }`
   - `JumpWouldCauseNegativeDF { date: Date, jump_bps: f64 }`
7. [ ] 単体テスト
   - jump_pillars なしでデシリアライズ → 空リスト
   - 重複日付 → エラー
   - 無効 confidence → エラー
   - 既存 API 後方互換性

**Acceptance Criteria**:
- 既存 CurveDefinition JSON が変更なしでデシリアライズ可能
- バリデーションエラーが構造化されている

---

## Phase 2: pricer_models 層（実装）

### Task 6: BootstrappedCurve の拡張

**Objective**: BootstrappedCurve にジャンプ対応の discount factor 計算を追加する。

**Requirements**: 2.2-2.6, 5.1-5.5

**Files**:
- `crates/pricer_models/src/market.rs` — BootstrappedCurve 拡張

**Subtasks**:
1. [ ] `jumps: Vec<(T, T)>` フィールド追加（time, cumulative_offset）
2. [ ] `with_jumps()` Builder メソッド
3. [ ] `discount_factor_with_limit(t, Limit)` 実装
   - Limit::Left → ジャンプ直前値
   - Limit::Right → ジャンプ直後値
   - Limit::Continuous → 右極限（デフォルト）
4. [ ] `forward_rate_with_limit(t1, t2, Limit)` 実装
5. [ ] `decompose_forward_rate(t1, t2)` 実装
   - continuous, jump, total 成分分解
6. [ ] ジャンプ日の二分探索ヘルパー
7. [ ] 単体テスト
   - ジャンプなし → 既存と同一結果
   - 単一ジャンプ → 左右極限の差分確認
   - 複数ジャンプ → 累積オフセット検証
   - forward_rate 整合性（DF から計算 = zero rate から計算）

**Acceptance Criteria**:
- discount_factor_with_limit が正しく左右極限を返す
- ジャンプなしの場合、既存動作と完全一致

---

### Task 7: CurveBootstrapper の拡張

**Objective**: CurveBootstrapper が JumpPillar を考慮したキャリブレーションを実行できるようにする。

**Requirements**: 3.1-3.6

**Files**:
- `crates/pricer_models/src/builder/curve/bootstrap.rs`

**Subtasks**:
1. [ ] `bootstrap_with_definition()` メソッド追加
2. [ ] JumpPillar からジャンプリスト変換（Task 3 の関数使用）
3. [ ] キャリブレーション時のジャンプオフセット適用ロジック
4. [ ] `effective_jump_at(t, jumps)` メソッド実装
5. [ ] 複数ジャンプの集約（同一日付）
6. [ ] debug logging（適用ジャンプ情報）
7. [ ] 統合テスト
   - 単一ジャンプ付き曲線構築
   - 複数ジャンプ付き曲線構築
   - ジャンプを跨ぐ商品の再価格付け検証

**Acceptance Criteria**:
- JumpPillar 付き CurveDefinition からブートストラップ成功
- ジャンプを跨ぐ商品価格が正確

---

## Phase 3: 統合・後方互換性

### Task 8: 統合テストと後方互換性確認

**Objective**: 全コンポーネントの統合動作と後方互換性を検証する。

**Requirements**: 7.1-7.5

**Files**:
- `crates/pricer_models/tests/jump_aware_curve_integration.rs` — 新規

**Subtasks**:
1. [ ] JumpPillar なし CurveDefinition のブートストラップ → 既存結果一致
2. [ ] 既存 JSON 設定ファイルのデシリアライズ
3. [ ] EventInstrument → JumpPillarBuilder → CurveDefinition → CurveBootstrapper フロー
4. [ ] OIS スワップ・FRA のジャンプ考慮価格付け
5. [ ] パフォーマンス確認（ジャンプ検索 O(log n)）

**Acceptance Criteria**:
- 既存テストがすべてパス
- 新規統合テストがパス
- 性能劣化なし

---

## Task Summary

| Task | Phase | Component | Priority | Dependencies |
|------|-------|-----------|----------|--------------|
| 1 | 1 | Limit enum | P0 | - |
| 2 | 1 | JumpPillar | P0 | - |
| 3 | 1 | Date→Time 変換 | P0 | Task 2 |
| 4 | 1 | JumpPillarBuilder | P1 | Task 2 |
| 5 | 1 | CurveDefinition 拡張 | P0 | Task 2 |
| 6 | 2 | BootstrappedCurve 拡張 | P0 | Task 1, 3 |
| 7 | 2 | CurveBootstrapper 拡張 | P0 | Task 5, 6 |
| 8 | 3 | 統合テスト | P0 | Task 1-7 |

---

## Implementation Order

推奨実装順序（依存関係を考慮）:

1. **Task 1**: Limit enum（依存なし、基盤型）
2. **Task 2**: JumpPillar（依存なし、基盤構造体）
3. **Task 3**: Date→Time 変換（Task 2 依存、設計課題対応）
4. **Task 5**: CurveDefinition 拡張（Task 2 依存）
5. **Task 4**: JumpPillarBuilder（Task 2 依存、並行可能）
6. **Task 6**: BootstrappedCurve 拡張（Task 1, 3 依存）
7. **Task 7**: CurveBootstrapper 拡張（Task 5, 6 依存）
8. **Task 8**: 統合テスト（全タスク完了後）
