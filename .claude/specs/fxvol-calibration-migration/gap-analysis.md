# ギャップ分析: FXボラティリティキャリブレーション移行

## 1. エグゼクティブサマリー

### 分析範囲
FXボラティリティサーフェスのキャリブレーションロジックを `demo/gui` から `pricer_core` および `pricer_models` へ移行する。

### 主要な発見事項
1. **基盤は80%完成済み**: infra_domain の FX vol 型定義、pricer_core の SABR/Garman-Kohlhagen、pricer_models の vol builder 構造が既に存在
2. **移行が必要なロジック**: demo_gui の `delta_to_strike` 関数、RR/BF 変換ロジック、フォワード計算のインラインコード
3. **プレースホルダー実装**: `SabrSliceCalibrator::calibrate_slice` は初期推定値を返すのみで、実際の最適化は未実装
4. **FxCurve が未実装**: Interest Rate Parity に基づく FX フォワードカーブのトレイト/構造体が存在しない

### 推奨アプローチ
**ハイブリッドアプローチ**: 既存の型定義と構造を最大限活用しつつ、不足しているコンポーネントを追加実装する。

---

## 2. 既存コンポーネント分析

### 2.1 infra_domain (100% 完成)

**場所**: `crates/infra_domain/src/trade/instrument_def/fx_vol.rs`

| コンポーネント | 状態 | 説明 |
|---------------|------|------|
| `DeltaType` | ✅ 完成 | SpotDelta, PremiumAdjusted, ForwardDelta 列挙型 |
| `Delta` | ✅ 完成 | 0〜50 範囲の newtype、D10/D25/ATM 定数 |
| `FxVolConvention` | ✅ 完成 | 通貨ペアごとのデルタ慣行定義 |
| `FxVolInstrument` | ✅ 完成 | ATM, Butterfly, RiskReversal, DeltaQuoted 列挙型 |
| `FxVolInstrumentBuilder` | ✅ 完成 | ビルダーパターン実装 |

**評価**: 要件1-7で使用する型定義は全て揃っている。追加実装不要。

### 2.2 pricer_core (90% 完成)

**場所**: `crates/pricer_core/src/math/formulas/`

| コンポーネント | 状態 | 説明 |
|---------------|------|------|
| `sabr.rs` | ✅ 完成 | Hagan 公式、SabrImpliedVolParams、sabr_implied_vol() |
| `garman_kohlhagen.rs` | ✅ 完成 | GarmanKohlhagen モデル、delta() メソッド |
| `fx_delta.rs` | ❌ 未実装 | delta_to_strike、strike_to_delta 関数 |

**評価**: SABR インプライドボラティリティ計算と Garman-Kohlhagen の delta 計算は完成している。
delta_to_strike 関数は demo_gui に実装があるが、pricer_core へ移行が必要。

### 2.3 pricer_models (60% 完成)

**場所**: `crates/pricer_models/src/builder/vol/`

| コンポーネント | 状態 | 説明 |
|---------------|------|------|
| `SabrParams` | ✅ 完成 | α, β, ρ, ν パラメータ構造体 |
| `VolQuote` | ✅ 完成 | strike, volatility, forward フィールド |
| `SliceCalibrationConfig` | ✅ 完成 | fixed_beta, max_iterations, tolerance |
| `SabrSliceCalibrator` | ⚠️ プレースホルダー | calibrate_slice は初期推定値のみ返す |
| `FxVolBuilder` | ⚠️ 部分実装 | add_quote/calibrate は動作するが FxVolInstrument 未対応 |
| `FxVolResult` | ⚠️ 部分実装 | diagnostics フィールドなし |

**場所**: `crates/pricer_models/src/market.rs`

| コンポーネント | 状態 | 説明 |
|---------------|------|------|
| `YieldCurve` trait | ✅ 完成 | discount_factor, zero_rate, forward_rate |
| `FlatCurve` | ✅ 完成 | 定数金利カーブ |
| `BootstrappedCurve` | ✅ 完成 | ピラー補間カーブ |
| `FxCurve` trait | ❌ 未実装 | FX フォワード計算トレイト |
| `IrpFxCurve` | ❌ 未実装 | Interest Rate Parity FX カーブ |

### 2.4 demo_gui (移行対象)

**場所**: `demo/gui/src/web/handlers/fxvol.rs`

| コンポーネント | 移行先 | 説明 |
|---------------|--------|------|
| `FxQuoteEntry::to_delta_vols()` | pricer_models | RR/BF → デルタボラティリティ変換 |
| `DeltaVols` struct | pricer_models | 5点デルタボラティリティ構造体 |
| `delta_to_strike()` | pricer_core | デルタ→ストライク変換関数 |
| `norm_inv()` | pricer_core | 逆正規分布関数（インライン実装） |
| フォワード計算 | pricer_models | IrpFxCurve で置き換え |
| `calibrate_surface` handler | demo_gui | FxVolBuilder を使用するよう簡略化 |

---

## 3. 要件別ギャップ分析

### 要件1: Delta-Strike変換関数

**ギャップ**: `delta_to_strike` と `strike_to_delta` が pricer_core に存在しない

**既存資産**:
- demo_gui に `delta_to_strike` 関数が実装済み（約90行）
- `norm_inv` 関数（逆正規分布）もインライン実装済み
- pricer_core に `norm_cdf`, `norm_pdf` が既存（`distributions.rs`）
- Garman-Kohlhagen に `delta()` メソッドが既存

**実装オプション**:

| オプション | 説明 | 工数 | リスク |
|-----------|------|------|--------|
| A: 移植 | demo_gui のコードを pricer_core へ移植 | 低 | 低 |
| B: 拡張 | GarmanKohlhagen に `strike_from_delta()` を追加 | 中 | 低 |
| C: 新規 | 完全な FX delta 計算モジュールを新規作成 | 高 | 低 |

**推奨**: オプション A（移植）- 既存の実装を活用し、テストを追加

### 要件2: FXフォワードカーブ

**ギャップ**: `FxCurve` トレイトと `IrpFxCurve` 構造体が存在しない

**既存資産**:
- `YieldCurve` トレイトが market.rs に定義済み
- demo_gui にフォワード計算のインラインコード: `spot * (rate_diff * expiry).exp()`

**実装オプション**:

| オプション | 説明 | 工数 | リスク |
|-----------|------|------|--------|
| A: 最小実装 | FxCurve トレイト + IrpFxCurve のみ | 低 | 低 |
| B: 汎用実装 | FX スポットカーブ、フォワードカーブ分離 | 中 | 中 |
| C: 完全実装 | FX ボラティリティサーフェス連携含む | 高 | 中 |

**推奨**: オプション A（最小実装）- 要件に必要十分な実装

### 要件3: RR/BF変換ロジック

**ギャップ**: `DeltaVolSlice` と `from_rr_bf` が pricer_models に存在しない

**既存資産**:
- demo_gui に `FxQuoteEntry::to_delta_vols()` が実装済み
- demo_gui に `DeltaVols` 構造体が定義済み
- 変換式は標準的: `vol_call = atm + bf + rr/2`, `vol_put = atm + bf - rr/2`

**実装オプション**:

| オプション | 説明 | 工数 | リスク |
|-----------|------|------|--------|
| A: 移植 | demo_gui から pricer_models へ移植 | 低 | 低 |
| B: 拡張 | VolQuote を拡張して RR/BF 対応 | 中 | 中 |

**推奨**: オプション A（移植）- demo_gui の実装は正確で十分テスト済み

### 要件4: SABRキャリブレーション

**ギャップ**: `SabrSliceCalibrator::calibrate_slice` がプレースホルダー実装

**既存資産**:
- `SabrSliceCalibrator` の構造体とトレイト実装が存在
- `SliceCalibrationConfig` で max_iterations, tolerance が定義済み
- pricer_core の `sabr_implied_vol` が利用可能
- Levenberg-Marquardt ソルバーが workspace に存在（`levenberg-marquardt` クレート）

**実装オプション**:

| オプション | 説明 | 工数 | リスク |
|-----------|------|------|--------|
| A: LM直接 | levenberg-marquardt クレートを直接使用 | 中 | 低 |
| B: 内製 | pricer_core のソルバーを使用 | 中 | 中 |
| C: 簡易 | Newton-Raphson で最適化 | 低 | 中 |

**推奨**: オプション A（LM直接）- 既存の依存関係を活用、収束特性が良好

### 要件5: FxVolBuilder拡張

**ギャップ**: `FxVolBuilder` が `FxVolInstrument` を直接受け取れない

**既存資産**:
- `FxVolBuilder` の基本構造が存在（slices, config, calibrator）
- `add_quote` メソッドが実装済み
- `calibrate` メソッドが実装済み

**実装オプション**:

| オプション | 説明 | 工数 | リスク |
|-----------|------|------|--------|
| A: 拡張 | with_fx_curve, with_convention, add_instrument を追加 | 中 | 低 |
| B: ラッパー | FxVolInstrumentAdapter を作成 | 中 | 低 |

**推奨**: オプション A（拡張）- FxVolBuilder を直接拡張

### 要件6: キャリブレーション診断情報

**ギャップ**: `FxVolResult` に diagnostics フィールドがない

**既存資産**:
- demo_gui に `FxCalibrationDiagnostics` が定義済み
- demo_gui に `SabrParameters` が定義済み（residual, iterations 含む）

**実装オプション**:

| オプション | 説明 | 工数 | リスク |
|-----------|------|------|--------|
| A: 追加 | SliceCalibrationDiagnostics を新規追加、FxVolResult を拡張 | 低 | 低 |
| B: 統合 | SabrParams に診断情報を統合 | 低 | 低 |

**推奨**: オプション A（追加）- 関心の分離を維持

### 要件7: demo_gui簡略化

**ギャップ**: demo_gui がキャリブレーションロジックを直接実装

**依存関係**: 要件1-6が完了している必要あり

**実装オプション**:

| オプション | 説明 | 工数 | リスク |
|-----------|------|------|--------|
| A: 段階的 | 機能ごとに移行・テスト | 中 | 低 |
| B: 一括 | 全ロジックを一度に移行 | 中 | 中 |

**推奨**: オプション A（段階的）- リグレッションリスクを最小化

---

## 4. 技術的考慮事項

### 4.1 型システム整合性

**課題**: pricer_core と pricer_models で Float ジェネリクスを使用

**対応**:
- 新規関数も `<T: Float>` ジェネリクスを採用
- infra_domain の DeltaType は非ジェネリック（これは問題なし）

### 4.2 依存関係

**現在の依存グラフ**:
```
infra_domain ← pricer_core ← pricer_models ← demo_gui
```

**変更後も同じ**:
- pricer_core は infra_domain の DeltaType を使用（新規依存）
- pricer_models は pricer_core の delta_to_strike を使用
- demo_gui は pricer_models の FxVolBuilder を使用

**A-I-P-S 準拠**: 変更後も依存ルールに違反しない

### 4.3 Levenberg-Marquardt 統合

**既存状況**:
- `levenberg-marquardt` クレートが workspace に存在
- pricer_core にも最適化ソルバーが存在

**推奨**:
- `levenberg-marquardt` クレートを pricer_models から直接使用
- `LeastSquaresProblem` トレイトを SABR 残差関数に実装

### 4.4 テスト戦略

**往復変換テスト**:
- delta_to_strike → strike_to_delta の往復で一致確認
- 各 DeltaType（SpotDelta, ForwardDelta, PremiumAdjusted）でテスト

**SABR キャリブレーションテスト**:
- 既知のパラメータからクォートを生成 → キャリブレーション → パラメータ一致確認
- 収束失敗ケースのエラーハンドリング確認

---

## 5. 実装順序と依存関係

```
[1] Delta-Strike変換 (pricer_core)
    ↓
[2] FXフォワードカーブ (pricer_models) ─────┐
    ↓                                      │
[3] RR/BF変換 (pricer_models) ←─────────────┤
    ↓                                      │
[4] SABRキャリブレーション (pricer_models) ←┘
    ↓
[5] FxVolBuilder拡張 (pricer_models)
    ↓
[6] 診断情報 (pricer_models)
    ↓
[7] demo_gui簡略化 (demo_gui)
```

**推奨実装順序**: 1 → 2 → 4 → 3 → 5 → 6 → 7

**理由**:
- タスク4（SABR最適化）は独立して実装可能
- タスク3（RR/BF変換）はタスク1のみに依存
- タスク5は1,2,3に依存するため後回し

---

## 6. リスク評価

| リスク | 影響度 | 発生確率 | 軽減策 |
|--------|--------|----------|--------|
| SABR キャリブレーション収束失敗 | 中 | 低 | 初期推定値の改善、境界条件の追加 |
| PremiumAdjusted delta 計算の複雑さ | 低 | 中 | 反復ソルバーの使用 |
| demo_gui リグレッション | 中 | 低 | 段階的移行、既存テスト維持 |
| パフォーマンス劣化 | 低 | 低 | ベンチマーク比較 |

---

## 7. 結論

### 実装可能性: 高

既存資産が充実しており、主にコードの移動と接続が中心。新規アルゴリズム実装は SABR キャリブレーションのみ。

### 推奨アクション

1. **即時開始可能**: タスク1（delta_to_strike）とタスク4（SABR最適化）は並行して開始可能
2. **テスト先行**: 各タスクで既存の demo_gui テストを pricer_models に移植
3. **段階的移行**: demo_gui は機能ごとに移行し、リグレッションを監視

### 設計フェーズへの推奨事項

- delta_to_strike の API 設計を Garman-Kohlhagen と整合させる
- FxCurve トレイトを YieldCurve と類似の設計にする
- SliceCalibrationDiagnostics を CalibrationError と統合検討
