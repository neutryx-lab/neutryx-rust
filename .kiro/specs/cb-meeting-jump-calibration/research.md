# Research & Design Decisions: CB Meeting Jump Calibration

---
**Purpose**: CB Meeting日のフォワードレートジャンプをGlobalBootstrapperに統合するための調査結果と設計決定を記録する。

**Usage**: gap-analysis.mdの調査結果を基に、設計決定の詳細とトレードオフを文書化。
---

## Summary

- **Feature**: `cb-meeting-jump-calibration`
- **Discovery Scope**: Extension（既存GlobalBootstrapperの拡張）
- **Key Findings**:
  - 既存インフラ（GlobalBootstrapper, CalibrationMatrix, MarketEvent）は高い再利用性を持つ
  - ジャンプ処理の数学的モデルはDF乗算方式（DF × Π(1 + jump_i)）が最適
  - Newton-Raphson収束性リスクはdamping_factorとフォールバック戦略で軽減可能

## Research Log

### ジャンプパラメータの数学的表現

- **Context**: CB Meeting日におけるフォワードレートのジャンプをどのように数学的に表現するか
- **Sources Consulted**:
  - 既存`GlobalBootstrapper`のNewton-Raphson実装（`global.rs`）
  - `CalibrationProblem`のJacobian計算ロジック（`problem.rs`）
  - 金利デリバティブ実務におけるOIS Turn効果の標準的処理
- **Findings**:
  - ディスカウントファクターへのジャンプ適用: `DF_adjusted(t) = DF(t) × Π_{i: t_i < t} (1 + jump_i × dt_i)`
  - `jump_i`はbps単位、`dt_i`はジャンプ期間（通常O/N = 1/365）
  - Jacobian計算時、ジャンプパラメータの偏微分は`∂F/∂jump = Σ(cashflow × DF × dt)`
- **Implications**:
  - パラメータベクトルを拡張: `x = [log(DF_1), ..., log(DF_n), jump_1, ..., jump_m]`
  - Jacobian行列は`(n+m) × (n+m)`に拡張される

### ジャンプピラーと補間の不連続性

- **Context**: InterpolationMatrixがスムース補間前提である問題への対応
- **Sources Consulted**:
  - `InterpolationMatrix::from_pillars`の実装（`matrix.rs`）
  - Log-Linear補間のロジック
- **Findings**:
  - 現在の補間: `log(DF(t)) = w × log(DF_k) + (1-w) × log(DF_{k+1})`
  - ジャンプ対応補間: ジャンプ日を境界として補間区間を分割
  - ジャンプ前後で別々の補間セグメントを使用
- **Implications**:
  - `InterpolationMatrix::with_jump_pillars()`メソッドの追加が必要
  - ジャンプピラーはグリッドに自動追加されるが、パラメータとしては別管理

### 収束性とフォールバック戦略

- **Context**: ジャンプパラメータ追加がNewton-Raphson収束に与える影響
- **Sources Consulted**:
  - `GlobalBootstrapConfig`のdamping_factor設定
  - condition_number監視ロジック
- **Findings**:
  - ジャンプパラメータ追加でJacobian行列の条件数が悪化する可能性
  - damping_factor適用で安定化可能（0.5〜0.8推奨）
  - 収束失敗時、ジャンプを無視した再カリブレーションが有効
- **Implications**:
  - `GlobalBootstrapConfig::with_jump_fallback(true)`オプション追加
  - 収束失敗時の自動フォールバックロジック実装

### API拡張パターン

- **Context**: 既存`CurveBuildRequest`への後方互換な拡張方法
- **Sources Consulted**:
  - `curves.rs`の`CurveBuildRequest`定義
  - Serde JSONデシリアライゼーションパターン
- **Findings**:
  - `#[serde(default)]`属性でオプショナルフィールド追加可能
  - ネストされた`CbEventInput`構造体でジャンプ情報を受け取る
- **Implications**:
  - 既存リクエストはそのまま動作（後方互換）
  - 新規フィールド`cb_events: Option<Vec<CbEventInput>>`

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| 既存拡張 | GlobalBootstrapConfig/CalibrationProblemを直接拡張 | コード重複なし、テスト再利用 | global.rsが1000行超に肥大化 | シンプルだが保守性懸念 |
| 新規コンポーネント | JumpAwareBootstrapperを新規作成 | 関心の分離明確 | コード重複、テスト重複 | オーバーエンジニアリング |
| **ハイブリッド** | 設定・結果型を拡張、ジャンプロジック分離 | バランス良好、段階的導入可 | フェーズ管理の複雑さ | **採用** |

## Design Decisions

### Decision: ジャンプパラメータの単位

- **Context**: ジャンプ幅を何単位で表現するか
- **Alternatives Considered**:
  1. Absolute rate（0.0025 = 25bp）— 計算シンプルだがUI直感性低い
  2. Basis points（25 = 25bp）— UI入力直感的、変換要
  3. Percentage（0.25% = 25bp）— 曖昧さあり
- **Selected Approach**: Basis points (bps)
- **Rationale**: トレーダー/クオンツが日常的に使用する単位、UI入力との親和性
- **Trade-offs**: 内部計算時に0.0001倍への変換が必要
- **Follow-up**: バリデーション範囲は±100bpsとする

### Decision: 複数ジャンプの累積方法

- **Context**: 複数のCB Meeting日を跨ぐ商品での累積効果
- **Alternatives Considered**:
  1. 加算（DF × (1 + Σjump_i)）— シンプルだが数学的に不正確
  2. 乗算（DF × Π(1 + jump_i)）— 正確な複利効果
- **Selected Approach**: 乗算方式
- **Rationale**: 金融数学的に正確、複利効果を適切にモデル化
- **Trade-offs**: 計算がやや複雑、Jacobianの偏微分も複雑化
- **Follow-up**: テストケースで累積効果の検証

### Decision: フォールバック戦略

- **Context**: ジャンプ付きカリブレーションが収束しない場合の対応
- **Alternatives Considered**:
  1. エラーを返して終了 — ユーザー体験が悪い
  2. ジャンプなしで再試行 — 自動回復、結果は妥協
  3. damping増加で再試行 — 時間がかかる
- **Selected Approach**: ジャンプなしで再試行 + 警告返却
- **Rationale**: ユーザーに何らかの結果を提供しつつ、制限を明示
- **Trade-offs**: ジャンプなし結果はユーザーの意図と異なる可能性
- **Follow-up**: 警告メッセージに「ジャンプパラメータは無視されました」を含める

### Decision: ジャンプピラーのグリッド統合

- **Context**: ジャンプ日を通常のピラーと同様に扱うか分離するか
- **Alternatives Considered**:
  1. 完全統合（ピラー配列に追加）— シンプルだが次元増加
  2. 完全分離（別配列で管理）— 明確だが補間ロジック複雑化
  3. ハイブリッド（グリッドには追加、パラメータは別管理）
- **Selected Approach**: ハイブリッド
- **Rationale**: グリッドへの追加で補間精度を確保しつつ、パラメータベクトルは拡張
- **Trade-offs**: 実装がやや複雑
- **Follow-up**: `JumpPillar`構造体で日付とパラメータインデックスを管理

## Risks & Mitigations

- **Newton-Raphson収束性悪化** — damping_factor自動調整、ジャンプなしフォールバック
- **Jacobian数値不安定** — Central Difference使用、condition number上限監視
- **後方互換性破壊** — 全新規フィールドをOptional、デフォルトfalse
- **UI不連続表示の混乱** — トグルでジャンプ有効/無効切替、明確なマーカー表示

## References

- `crates/pricer_models/src/builder/curve/global.rs` — GlobalBootstrapper実装
- `crates/pricer_models/src/builder/problem.rs` — CalibrationProblem、Jacobian計算
- `crates/pricer_models/src/builder/matrix.rs` — InterpolationMatrix
- `crates/infra_master/src/market/events/market_event.rs` — MarketEvent定義
- `demo/gui/src/web/handlers/curves.rs` — REST APIハンドラ
- gap-analysis.md — 詳細なコンポーネント分析
