# Research & Design Decisions

---
**Purpose**: 技術設計を導くディスカバリー調査結果、アーキテクチャ調査、およびその根拠を記録する。

**Usage**:
- ディスカバリーフェーズ中の調査活動とその結果を記録
- `design.md` に含めるには詳細すぎる設計決定のトレードオフを文書化
- 将来の監査や再利用のための参照とエビデンスを提供
---

## Summary
- **Feature**: `frictionalbank-irs-bootstrap-risk`
- **Discovery Scope**: Extension（既存モジュールの統合・拡張）
- **Key Findings**:
  - Bootstrap/IrsGreeks バックエンドモジュールは完全実装済み
  - WebAppインフラ（Axum, WebSocket, Chart.js）は成熟
  - Option C（ハイブリッドアプローチ）が最適

## Research Log

### Bootstrap モジュール調査

- **Context**: イールドカーブ構築機能の既存実装を確認
- **Sources Consulted**:
  - `crates/pricer_optimiser/src/bootstrapping/mod.rs`
  - `crates/pricer_optimiser/src/bootstrapping/engine.rs`
- **Findings**:
  - `SequentialBootstrapper<T: Float>` - メインブートストラップエンジン
  - `GenericBootstrapResult<T>` - curve, pillars, discount_factors, residuals, iterations を含む
  - `BootstrappedCurve<T>` - `YieldCurve<T>` trait を実装
  - `BootstrapInstrument<T>` - OIS, IRS, FRA, Futures をサポート
  - `SensitivityBootstrapper` - AAD対応の感応度計算
  - `CachedBootstrapper<T>` - 繰り返しブートストラップの最適化
- **Implications**:
  - バックエンドAPIはこれらの型を直接使用可能
  - ジェネリック型 `<T: Float>` によりAAD互換

### IRS Greeks モジュール調査

- **Context**: リスク計算機能の既存実装を確認
- **Sources Consulted**:
  - `crates/pricer_pricing/src/irs_greeks/mod.rs`
  - `crates/pricer_pricing/src/irs_greeks/benchmark.rs`
- **Findings**:
  - `IrsGreeksCalculator<T>` - NPV, DV01 計算
  - `BenchmarkRunner` - AAD vs Bump-and-Revalue のベンチマーク
  - `BenchmarkConfig` - ウォームアップ、イテレーション設定
  - `TimingStats` - mean, std_dev, min, max 統計
  - `DeltaBenchmarkResult` - AAD/Bump 両方の結果と speedup_ratio
  - `ScalabilityResult` - テナー数に対するスケーラビリティ測定
  - JSON/Markdown 出力フォーマッター実装済み
- **Implications**:
  - ベンチマーク機能は要件6を完全にカバー
  - 既存の `TimingStats` 構造をAPIレスポンスに直接マッピング可能

### WebApp バックエンドパターン調査

- **Context**: 既存APIパターンを確認して一貫性を確保
- **Sources Consulted**:
  - `demo/gui/src/web/handlers.rs`
  - `demo/gui/src/web/pricer_types.rs`
  - `demo/gui/src/web/mod.rs`
  - `demo/gui/src/web/websocket.rs`
- **Findings**:
  - **ハンドラパターン**: `async fn handler(State(state): State<AppState>, Json(req)) -> Result<Json<Res>, ErrorType>`
  - **API型**: `#[derive(Deserialize)]` for Request, `#[derive(Serialize)]` for Response
  - **命名規則**: camelCase JSON シリアライゼーション
  - **エラー処理**: `PricingErrorResponse` 型、HTTP 400/422/500
  - **ルーティング**: `Router::new().route("/api/...", post(handler))`
  - **WebSocket**: `BroadcastMessage` enum、`state.tx.send()` でブロードキャスト
- **Implications**:
  - 既存パターンに厳密に従うことで一貫性を維持
  - `AppState` を通じたカーブ状態共有が必要

### Feature Flag 調査

- **Context**: AAD機能の条件分岐方法を確認
- **Sources Consulted**: `Cargo.toml` ファイル群
- **Findings**:
  - `enzyme-ad` feature: Enzyme LLVM-level 自動微分を有効化
  - `l1l2-integration` feature: L1/L2 統合を有効化
  - 条件分岐: `#[cfg(feature = "enzyme-ad")]`
- **Implications**:
  - AAD無効時はBump法のみ提供するフォールバック設計が必要
  - Feature状態をAPIレスポンスで明示

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| **A: 既存拡張** | 既存 handlers.rs を拡張 | 最小限の新規ファイル、既存パターン準拠 | index.html/app.js 肥大化 | シンプルだが保守性に懸念 |
| **B: 新規モジュール** | 専用ファイル群を新規作成 | 関心分離が明確 | 新規ファイル数多、共通コード重複 | オーバーエンジニアリング |
| **C: ハイブリッド** | バックエンド拡張 + フロントエンド適度分離 | バランス、既存活用、IIFE名前空間分離 | 中程度の変更箇所 | **推奨** |

## Design Decisions

### Decision: ハイブリッドアーキテクチャの採用

- **Context**: フロントエンドとバックエンドの変更戦略を決定
- **Alternatives Considered**:
  1. Option A — 既存ファイルへの直接追加（最小変更）
  2. Option B — 完全な新規モジュール作成（最大分離）
  3. Option C — バックエンド拡張 + フロントエンドIIFE分離
- **Selected Approach**: Option C（ハイブリッドアプローチ）
- **Rationale**:
  - 既存の `handlers.rs`, `pricer_types.rs` パターンを最大限活用
  - フロントエンドは IIFE パターンで名前空間を分離しつつ既存構造に統合
  - Bootstrap/IrsGreeks モジュールとの自然な統合
- **Trade-offs**:
  - ✅ 既存パターンとの一貫性
  - ✅ 過度な肥大化を防止
  - ❌ 中程度の変更箇所数
- **Follow-up**:
  - フロントエンドコードのIIFE構造を実装時に検証

### Decision: カーブ状態管理方式

- **Context**: Bootstrap結果をPricing/Risk計算で共有する方法
- **Alternatives Considered**:
  1. ステートレスAPI（毎回カーブデータを送信）
  2. セッションベースのカーブID管理
  3. インメモリ AppState でのカーブキャッシュ
- **Selected Approach**: インメモリ AppState + カーブID
- **Rationale**:
  - 既存の `AppState` パターンと整合
  - 複数リクエスト間でカーブを効率的に再利用
  - WebSocketブロードキャストと自然に連携
- **Trade-offs**:
  - ✅ 高パフォーマンス（ネットワーク転送削減）
  - ✅ 既存パターンとの一貫性
  - ❌ サーバー再起動でカーブ喪失（許容可能）
- **Follow-up**:
  - `RwLock<HashMap<CurveId, BootstrappedCurve>>` 実装

### Decision: AAD フォールバック戦略

- **Context**: `enzyme-ad` feature 無効時の動作
- **Alternatives Considered**:
  1. AAD API をエラーで拒否
  2. Bump法で代替計算（透過的フォールバック）
  3. API レスポンスでモード明示
- **Selected Approach**: API レスポンスでモード明示 + 機能制限通知
- **Rationale**:
  - ユーザーに現在の計算モードを明確に伝達
  - AAD未対応時も Bump 法で完全な機能を提供
  - 速度比較のデモ価値を維持
- **Trade-offs**:
  - ✅ 透明性（ユーザーがモードを認識）
  - ✅ 機能完全性
  - ❌ AAD真の性能を体験できない（feature依存）
- **Follow-up**:
  - レスポンスに `aad_available: bool` フィールド追加

## Risks & Mitigations

- **Risk 1: Bootstrap収束失敗** — 入力バリデーション強化、エラーメッセージで失敗テナーを明示
- **Risk 2: AAD feature 依存** — フォールバック設計、UI で AAD 利用可否を明示
- **Risk 3: カーブID管理の複雑化** — シンプルなUUID生成、TTL付きキャッシュクリーンアップ
- **Risk 4: フロントエンド肥大化** — IIFE パターンで名前空間分離、将来的なモジュール分割検討

## References

- [Axum Documentation](https://docs.rs/axum/) — Web framework patterns
- [Serde JSON](https://serde.rs/) — Serialization patterns (camelCase)
- [Chart.js](https://www.chartjs.org/docs/) — Visualization library
- `crates/pricer_optimiser/src/bootstrapping/` — Bootstrap module API
- `crates/pricer_pricing/src/irs_greeks/` — IRS Greeks module API
- `.kiro/specs/frictionalbank-irs-bootstrap-risk/gap-analysis.md` — Gap analysis document

---

_作成日: 2026-01-14_
_言語: 日本語 (spec.json.language: ja)_
