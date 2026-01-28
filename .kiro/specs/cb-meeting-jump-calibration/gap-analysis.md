# Gap Analysis: CB Meeting Jump Calibration

## 1. 現状調査

### 1.1 ドメイン関連アセットのスキャン

#### キーファイル/モジュール

| コンポーネント | パス | 状態 |
|--------------|------|------|
| MarketEvent | `crates/infra_master/src/market/events/market_event.rs` | ✅ 完全 |
| EventType | `crates/infra_master/src/market/events/event_type.rs` | ✅ CentralBankMeeting対応済 |
| EventImportance | `crates/infra_master/src/market/events/importance.rs` | ✅ Low/Medium/High/Critical |
| GlobalBootstrapper | `crates/pricer_models/src/builder/curve/global.rs` | ✅ 788行、Newton-Raphson実装済 |
| CalibrationProblem | `crates/pricer_models/src/builder/problem.rs` | ✅ SystemOfEquations実装 |
| CalibrationMatrix | `crates/pricer_models/src/builder/matrix.rs` | ✅ N×Mマトリックス |
| InterpolationMatrix | `crates/pricer_models/src/builder/matrix.rs` | ✅ ピラー→グリッド補間 |
| API Handler | `demo/gui/src/web/handlers/curves.rs` | ✅ build_curve実装済 |
| CB Meeting Data | `demo/data/input/events/central_bank_meetings.json` | ✅ FED/ECB/BOJ/BOE/SNB/RBA/BOC |

#### 再利用可能なコンポーネント

| コンポーネント | 再利用性 | 備考 |
|--------------|---------|------|
| GlobalBootstrapConfig | 高 | builderパターン、拡張容易 |
| GlobalBootstrapResult | 高 | jacobian_inverse格納済 |
| CalibrationInstrument trait | 高 | `pricing_error`メソッド |
| InterpolationMatrix | 中 | 補間重み計算、ジャンプ対応要拡張 |
| CurveDataLoader | 高 | JSONファイル読み込み |
| get_central_bank_meetings | 高 | 既存API、通貨別グルーピング |

#### アーキテクチャパターン

- **A-I-P-S階層**: 厳守（Infra→Pricer方向のみ）
- **CalibrationInstrument trait**: 各商品のpricing_error実装
- **Builder Pattern**: GlobalBootstrapConfigのwith_*メソッド群
- **SystemOfEquations trait**: Newton-Raphson用インターフェース

---

### 1.2 規約抽出

#### 命名規約
- 英国英語: `optimiser`, `serialisation`, `calibration`
- snake_case (モジュール/関数), PascalCase (型/trait)
- `Calibration*`接頭辞: カリブレーション関連

#### 依存方向
```text
infra_master (MarketEvent) → pricer_models (GlobalBootstrapper)
pricer_models → demo/gui (API Handler)
```

#### テスト配置
- 同一ファイル内 `#[cfg(test)]` モジュール
- 統合テスト: `crates/pricer_models/tests/global_solver_integration.rs`

---

### 1.3 統合サーフェス

#### データモデル
- `MarketEvent`: expected_jump_bpsフィールド追加要
- `GlobalBootstrapConfig`: ジャンプ関連設定追加要
- `GlobalBootstrapResult`: ジャンプ情報追加要

#### API
- `POST /api/curves/build`: cb_eventsパラメータ追加要
- `GET /api/curves/central-bank-meetings`: 既存利用可

#### フロントエンド
- `demo/gui/static/` 内のJavaScript: カーブ描画拡張要

---

## 2. 要件実現可能性分析

### 2.1 技術的ニーズ

| 要件 | 技術的ニーズ | 難易度 |
|-----|------------|-------|
| Req1: 期待ジャンプ幅入力 | MarketEvent構造体拡張、UI入力フィールド | 低 |
| Req2: GlobalSolverジャンプ対応 | 補間ロジック修正、Jacobian計算拡張 | 高 |
| Req3: CashflowMatrix統合 | 新規ジャンプピラー列追加 | 中 |
| Req4: API拡張 | リクエスト/レスポンス型追加 | 低 |
| Req5: WebUI表示 | Chart.js マーカー追加、ツールチップ | 中 |
| Req6: バリデーション | エラー型追加、フォールバック実装 | 低 |
| Req7: 後方互換性 | Optionalフィールド、デフォルトfalse | 低 |

### 2.2 ギャップ識別

#### Missing（未実装）

| ギャップ | 詳細 | 影響度 |
|---------|------|-------|
| expected_jump_bps | MarketEventに存在しない | 高 |
| ジャンプピラー処理 | GlobalBootstrapperに未実装 | 高 |
| 不連続補間 | InterpolationMatrixがスムース前提 | 高 |
| ジャンプ対応Jacobian | 追加パラメータの偏微分 | 中 |
| API cb_events | CurveBuildRequestに未実装 | 中 |
| フォワードカーブジャンプ表示 | フロントエンド未対応 | 中 |

#### Unknown（要調査）

| 項目 | 詳細 | 優先度 |
|-----|------|-------|
| ジャンプサイズの単位 | bps vs absolute rate | Research Needed |
| 複数ジャンプの累積効果 | 乗算 vs 加算 | Research Needed |
| ジャンプタイミング | 日付の年変換精度 | Research Needed |
| Jacobian安定性 | ジャンプ追加時の収束性 | Research Needed |

#### Constraint（制約）

| 制約 | 詳細 |
|-----|------|
| A-I-P-S依存方向 | infra_master→pricer_modelsの方向のみ |
| 後方互換性 | 既存APIブレーク禁止 |
| demo/guiステータス | 現在feature-gated、calibration依存 |

### 2.3 複雑性シグナル

- **アルゴリズムロジック**: Newton-Raphson Jacobian計算の拡張（高）
- **補間ロジック**: 不連続点での補間重み調整（中）
- **統合ワークフロー**: データ→API→カリブレーション→UI表示（中）
- **外部統合**: なし（内部完結）

---

## 3. 実装アプローチオプション

### Option A: 既存コンポーネント拡張

**対象ファイル修正**:

| ファイル | 変更内容 |
|---------|---------|
| `market_event.rs` | `expected_jump_bps: Option<f64>`追加 |
| `global.rs` | `GlobalBootstrapConfig`にジャンプ設定追加 |
| `problem.rs` | `CalibrationProblem`にジャンプピラー処理追加 |
| `matrix.rs` | `InterpolationMatrix`に不連続対応追加 |
| `curves.rs` (handler) | `CurveBuildRequest`にcb_events追加 |

**トレードオフ**:
- ✅ 新規ファイル最小化
- ✅ 既存テスト活用可能
- ❌ global.rsが複雑化（788行→1000行超見込み）
- ❌ 既存CalibrationInstrument trait変更の影響範囲大

### Option B: 新規コンポーネント作成

**新規ファイル**:

| ファイル | 責務 |
|---------|-----|
| `builder/curve/jump.rs` | JumpAwareBootstrapper, JumpConfig |
| `builder/jump_matrix.rs` | JumpAwareInterpolationMatrix |
| `handlers/curves_with_jumps.rs` | ジャンプ対応APIエンドポイント |

**トレードオフ**:
- ✅ 関心の分離明確
- ✅ 既存コード影響最小
- ❌ ファイル数増加
- ❌ 重複コード発生リスク

### Option C: ハイブリッドアプローチ（推奨）

**フェーズ1: データ層拡張**（影響最小）
- `MarketEvent`に`expected_jump_bps`追加（infra_master）
- `central_bank_meetings.json`スキーマ拡張不要（動的追加）

**フェーズ2: カリブレーション層拡張**
- `GlobalBootstrapConfig`にジャンプ設定追加
- 新規`JumpPillar`構造体をglobal.rs内に追加
- `CalibrationProblem`をジェネリック化してジャンプ対応

**フェーズ3: 補間層拡張**
- `InterpolationMatrix::with_jumps()`メソッド追加
- 既存`from_pillars`との互換維持

**フェーズ4: API/UI拡張**
- `CurveBuildRequest`にオプショナルcb_events追加
- フォワードカーブ表示にジャンプマーカー追加

**トレードオフ**:
- ✅ 段階的導入でリスク軽減
- ✅ 既存機能との互換維持
- ✅ フィーチャーフラグで段階的有効化可能
- ❌ 実装フェーズ管理の複雑さ

---

## 4. 実装複雑性とリスク

### 工数見積もり

| フェーズ | 工数 | 根拠 |
|---------|-----|------|
| フェーズ1: データ層 | S (1-2日) | 単純なフィールド追加 |
| フェーズ2: カリブレーション層 | L (1-2週間) | Jacobian計算、収束ロジック |
| フェーズ3: 補間層 | M (3-5日) | 数学的検証、テスト |
| フェーズ4: API/UI | M (3-5日) | 統合テスト、UI実装 |
| **合計** | **L-XL (2-3週間)** | |

### リスク評価

| リスク | レベル | 軽減策 |
|-------|-------|-------|
| Newton-Raphson収束性悪化 | 高 | damping_factor活用、ジャンプなしフォールバック |
| Jacobian数値不安定 | 中 | Central Difference使用、condition number監視 |
| 後方互換性破壊 | 低 | Optional型、デフォルトfalse |
| UI表示の不連続処理 | 低 | Chart.jsギャップ機能活用 |

---

## 5. 設計フェーズへの推奨事項

### 推奨アプローチ
**Option C: ハイブリッドアプローチ**を推奨

### キー設計決定事項
1. **ジャンプパラメータのスコープ**: GlobalBootstrapConfig内に配置
2. **ジャンプサイズの単位**: bps（0.01% = 1bp）で統一
3. **複数ジャンプの累積**: 乗算（DF_adjusted = DF × Π(1 + jump_i)）
4. **フィーチャーフラグ**: `feature = "jump-calibration"` for pricer_models

### リサーチ項目（設計フェーズで調査）
1. Jacobian計算時のジャンプ項の偏微分公式
2. ジャンプピラーと通常ピラーの重複処理
3. フォワードカーブ表示でのギャップ vs 接続線の選択基準
4. 収束失敗時のフォールバック戦略詳細

---

## 6. 要件-アセットマッピング

| 要件ID | 既存アセット | ギャップ | ステータス |
|--------|------------|---------|----------|
| Req1 | MarketEvent, EventImportance | expected_jump_bps | Missing |
| Req2 | GlobalBootstrapper, CalibrationProblem | ジャンプピラー処理 | Missing |
| Req3 | CalibrationMatrix, InterpolationMatrix | 不連続補間 | Missing |
| Req4 | CurveBuildRequest, build_curve | cb_events param | Missing |
| Req5 | Chart.js, interpolate_forward_rate | ジャンプマーカー | Missing |
| Req6 | ApiError, CalibrationError | JumpCalibrationFailed | Missing |
| Req7 | GlobalBootstrapConfig | jump_enabled: false default | Missing |
