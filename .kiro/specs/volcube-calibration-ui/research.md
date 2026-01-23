# Research & Design Decisions: volcube-calibration-ui

## Summary
- **Feature**: `volcube-calibration-ui`
- **Discovery Scope**: Extension（既存システム拡張）
- **Key Findings**:
  - VolCube/FxVolatilitySurfaceバックエンドは完全実装済み、API層のみ新規
  - `curve_builder_types.rs`/`curve_builder_handlers.rs`が参照実装として機能
  - FX確率密度計算とDelta-Strike変換がバックエンド側で未実装
  - 3D描画にはPlotly.jsを推奨（金融向けサンプル豊富、宣言的API）

## Research Log

### 既存Curve Builder実装パターン
- **Context**: VolCube/FxVol APIの設計パターンを確立するため
- **Sources Consulted**:
  - `demo/gui/src/web/curve_builder_types.rs` (876 LOC)
  - `demo/gui/src/web/curve_builder_handlers.rs`
- **Findings**:
  - 型定義は`*_types.rs`に分離、Serde derive + camelCase変換
  - RFC 7807 `ProblemDetails`でエラーレスポンス標準化
  - `InstrumentFile`構造体でJSONファイル読み込み
  - `CurveBuildRequest`/`CurveBuildResponse`パターン
  - Enum helper methods（`display_name()`, `is_recommended()`, `is_enabled()`）
- **Implications**: 同一パターンをVolCube/FxVol APIに適用可能

### VolCube バックエンド実装状況
- **Context**: API層設計に必要なバックエンドインターフェース確認
- **Sources Consulted**:
  - `crates/pricer_models/src/market/volcube/mod.rs`
  - `crates/pricer_models/src/market/volcube/types.rs`
  - `crates/pricer_models/src/market/volcube/config.rs`
  - `crates/pricer_models/src/market/volcube/builder.rs`
- **Findings**:
  - `VolCubeBuilder<T>`: Fluent API、`with_instruments()`, `with_config()`, `build()`
  - `VolCubeConfig`: `InterpolationMethod`, `ExtrapolationMethod`, `StrikeAxisType`, `OptimizerMethod`, SABR beta/shift
  - `VolInstrument<T>`: expiry, tenor, strike, implied_vol, forward, weight
  - `SabrParams<T>`: alpha, beta, rho, nu
  - `VolatilityCube` trait: `volatility()`, `probability_density()`, `cumulative_probability()`
  - `BreedenLitzenberger`: `probability_density()`, `cumulative_probability()`, `statistics()`
- **Implications**: バックエンドAPIは完備、型変換のみ必要

### FxVolatilitySurface バックエンド実装状況
- **Context**: FX専用機能のバックエンドサポート確認
- **Sources Consulted**:
  - `crates/pricer_models/src/market/surfaces/fx.rs` (761 LOC)
- **Findings**:
  - `FxVolatilitySurface<T>`: Delta × Expiry グリッド、Bilinear補間
  - `FxDeltaPoint` enum: Put10D, Put25D, Atm, Call25D, Call10D
  - `volatility_by_delta()`, `atm_volatility()`, `risk_reversal_25d()`, `butterfly_25d()` 実装済み
  - `VolatilitySurface<T>` trait実装（`volatility()` = `volatility_by_delta()`）
  - **未実装**: `probability_density()`, `delta_to_strike()` 変換
- **Implications**: FX確率密度にはバックエンド拡張が必要

### FX確率密度計算の実装オプション
- **Context**: Req 10.7, 10.8 のFX確率密度機能実現
- **Sources Consulted**:
  - `crates/pricer_models/src/market/volcube/breeden_litzenberger.rs`
  - Garman-Kohlhagen FX option pricing theory
- **Findings**:
  - 既存`BreedenLitzenberger`は`VolatilityCube<T>`トレイトに依存
  - FX確率密度計算には以下が必要:
    1. Delta → Strike変換（Garman-Kohlhagen逆算）
    2. Strike軸上でのvol補間
    3. 数値微分でPDF計算
  - Delta-Strike変換には: spot, domestic_rate, foreign_rate, expiry, vol が必要
- **Implications**:
  - Option 1: `FxVolatilitySurface`に`delta_to_strike()`, `probability_density()`追加
  - Option 2: 新モジュール`fx_density.rs`作成（推奨）
  - Option 3: `BreedenLitzenberger`をgeneric化して`VolatilitySurface`対応

### 3D描画ライブラリ選定
- **Context**: Req 7 の3Dボラティリティサーフェス可視化
- **Sources Consulted**:
  - Plotly.js公式ドキュメント（3D Surface Plot）
  - Three.js公式ドキュメント
  - 既存Chart.js統合（demo/gui/static/index.html）
- **Findings**:
  - **Plotly.js**:
    - 宣言的API（`Plotly.newPlot()`）
    - 金融向け3Dサーフェスサンプル豊富
    - カラーマップ（Viridis, Plasma等）内蔵
    - インタラクティブ回転/ズーム標準機能
    - CDN: `https://cdn.plot.ly/plotly-2.35.2.min.js`
  - **Three.js**:
    - 高度な3Dレンダリング
    - カスタマイズ性高い
    - 学習コスト大、ボイラープレート多い
- **Implications**: Plotly.js推奨（既存Chart.jsとの共存容易、金融向け）

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| **A: 既存パターン踏襲** | Curve Builder実装パターンをVolCube/FxVolに適用 | 一貫性、学習コスト最小、既存インフラ活用 | なし | **推奨** |
| B: 統合モジュール | Curve/VolCube/FxVolを統合キャリブレーションAPIに | コード重複削減 | 破壊的変更リスク、実装期間延長 | 将来検討 |
| C: ハイブリッド | 共通ユーティリティ抽出 | 段階的リファクタ | 共通化判断に時間 | 将来検討 |

## Design Decisions

### Decision: API型定義パターン
- **Context**: VolCube/FxVol API型の設計
- **Alternatives Considered**:
  1. 新規パターン設計
  2. curve_builder_types.rsパターン踏襲
- **Selected Approach**: curve_builder_types.rsパターン踏襲
- **Rationale**: 既存の`InterpolationMethod`, `ProblemDetails`パターンが確立済み
- **Trade-offs**: ファイルサイズ大きくなる可能性（許容範囲）
- **Follow-up**: 共通型の将来的な抽出を検討

### Decision: FX確率密度実装アプローチ
- **Context**: FxVolatilitySurfaceからの確率密度計算
- **Alternatives Considered**:
  1. `FxVolatilitySurface`に直接メソッド追加
  2. `BreedenLitzenberger`を汎用化
  3. 新モジュール`fx_density.rs`作成
- **Selected Approach**: Option 3（新モジュール作成）
- **Rationale**:
  - FX特有のDelta-Strike変換ロジックを分離
  - 既存`BreedenLitzenberger`（VolCube用）への影響なし
  - 単一責任原則に沿った設計
- **Trade-offs**: 一部コード重複（密度計算の数値微分部分）
- **Follow-up**: 将来的に共通密度計算utilの抽出を検討

### Decision: 3D描画ライブラリ
- **Context**: 3Dボラティリティサーフェス可視化
- **Alternatives Considered**:
  1. Plotly.js
  2. Three.js
  3. Chart.js + chartjs-plugin-3d
- **Selected Approach**: Plotly.js
- **Rationale**:
  - 宣言的API（Chart.jsと類似の使い勝手）
  - 金融向け3Dサーフェスサンプル豊富
  - カラーマップ・インタラクション標準搭載
- **Trade-offs**: バンドルサイズ（~3MB、CDN利用で軽減）
- **Follow-up**: パフォーマンステスト（1000点グリッド）

### Decision: Delta-Strike変換方式
- **Context**: FX DeltaからAbsolute Strikeへの変換
- **Alternatives Considered**:
  1. Spot Delta（premium excluded）
  2. Forward Delta
  3. Premium-adjusted Delta
- **Selected Approach**: Spot Delta（デフォルト）+ Forward Delta オプション
- **Rationale**: 市場慣行に沿った柔軟性
- **Trade-offs**: 複数モードのサポートによる複雑性増
- **Follow-up**: 通貨ペア別デフォルト設定の検討

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| FX確率密度の数値安定性 | Strike範囲制限、警告表示機能（Req 6.6対応） |
| キャリブレーション性能 | 既存LRUキャッシュ活用、非同期オプション検討 |
| 3Dレンダリングパフォーマンス | グリッド解像度制限（最大50×50）、LOD対応 |
| ジェネリック型→f64変換 | API層でf64固定、型安全性維持 |

## References
- [Plotly.js 3D Surface](https://plotly.com/javascript/3d-surface-plots/) — 3Dサーフェスプロット公式ドキュメント
- [Garman-Kohlhagen Model](https://en.wikipedia.org/wiki/Foreign_exchange_option) — FXオプション価格理論
- [Breeden-Litzenberger Formula](https://en.wikipedia.org/wiki/Breeden%E2%80%93Litzenberger_formula) — リスクニュートラル密度導出
- [RFC 7807 Problem Details](https://datatracker.ietf.org/doc/html/rfc7807) — HTTPエラーレスポンス標準
