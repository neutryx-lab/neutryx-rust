# Research & Design Decisions

## Summary
- **Feature**: `fxvol-calibration-migration`
- **Discovery Scope**: Extension（既存システムへの統合型拡張）
- **Key Findings**:
  - infra_domain の FX vol 型定義は100%完成しており、追加実装不要
  - pricer_core の SABR/Garman-Kohlhagen は完成済み、delta_to_strike のみ移植が必要
  - pricer_models の SabrSliceCalibrator はプレースホルダー実装、levenberg-marquardt クレートで置換
  - FxCurve トレイトと IrpFxCurve は新規実装が必要（YieldCurve パターンに準拠）

---

## Research Log

### Delta-Strike変換の数学的実装

- **Context**: demo_gui の delta_to_strike 関数を pricer_core へ移植する際の正確な数式確認
- **Sources Consulted**:
  - 既存実装: `demo/gui/src/web/handlers/fxvol.rs`
  - Garman-Kohlhagen モデル: `pricer_core/src/math/formulas/garman_kohlhagen.rs`
  - FX Delta Conventions (Wystup, Uwe. "FX Options and Structured Products")
- **Findings**:
  - SpotDelta: `d1 = Φ⁻¹(Δ / e^(-rf×T))`、ストライク `K = F × e^(-d1×σ×√T + 0.5×σ²×T)`
  - ForwardDelta: `d1 = Φ⁻¹(Δ)`、同じストライク計算式
  - PremiumAdjusted: 反復解法が必要（プレミアムがストライクに依存）
  - 逆正規分布関数 `norm_inv` が必要（Moro のアルゴリズムが一般的）
- **Implications**:
  - PremiumAdjusted は反復ソルバーを使用（Newton-Raphson または Brent）
  - strike_to_delta は GarmanKohlhagen::delta() を再利用可能

### Levenberg-Marquardt ソルバー統合

- **Context**: SabrSliceCalibrator の実装に LM アルゴリズムを使用
- **Sources Consulted**:
  - `levenberg-marquardt` crate: https://docs.rs/levenberg-marquardt
  - 既存の workspace 依存関係: `Cargo.toml`
- **Findings**:
  - `levenberg-marquardt` クレートが workspace に既存
  - `LeastSquaresProblem` トレイトを実装する必要あり
  - パラメータ数 = 3（α, ρ, ν）、残差数 = クォート数
  - 境界制約は投影法で処理（LM 自体は無制約）
- **Implications**:
  - SABR 残差関数を `LeastSquaresProblem` として実装
  - パラメータ境界は最適化後にクリッピング

### FxCurve トレイト設計

- **Context**: Interest Rate Parity に基づく FX フォワード計算のトレイト設計
- **Sources Consulted**:
  - 既存の YieldCurve トレイト: `pricer_models/src/market.rs`
  - FX フォワード計算: F = S × df_foreign / df_domestic
- **Findings**:
  - YieldCurve トレイトは `discount_factor(t)`, `zero_rate(t)`, `forward_rate(t1, t2)` を提供
  - FxCurve トレイトは `spot()`, `forward(expiry)`, `currency_pair()` を提供すべき
  - IrpFxCurve は2つの YieldCurve を保持（domestic, foreign）
- **Implications**:
  - FxCurve は market モジュールに fx_curve サブモジュールとして追加
  - ジェネリクス `<T: Float>` を使用

---

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| A: 直接移植 | demo_gui のコードを pricer_core/models へ直接移植 | 低工数、動作実績あり | コード品質の不整合 | **選択** |
| B: 新規設計 | 完全に新規設計・実装 | 最適化された設計 | 高工数、リスク | 却下 |
| C: 抽象化層 | アダプターパターンで既存コードをラップ | 柔軟性 | 複雑性増加 | 却下 |

---

## Design Decisions

### Decision: Delta-Strike変換のAPI設計

- **Context**: delta_to_strike 関数のシグネチャと配置場所
- **Alternatives Considered**:
  1. GarmanKohlhagen 構造体にメソッド追加 — 既存モデルと密結合
  2. 独立関数として fx_delta モジュールに配置 — 再利用性が高い
  3. FxDeltaConverter 構造体を作成 — 過剰な抽象化
- **Selected Approach**: Option 2 - fx_delta モジュールに独立関数として実装
- **Rationale**:
  - GarmanKohlhagen はプライシングモデル、delta_to_strike は変換ユーティリティ
  - 他のモジュール（pricer_models）から直接呼び出し可能
  - infra_domain の DeltaType を引数に取ることで型安全性を確保
- **Trade-offs**:
  - (+) 独立性が高く再利用可能
  - (-) GarmanKohlhagen との重複計算の可能性
- **Follow-up**: パフォーマンステストで重複計算の影響を確認

### Decision: SabrSliceCalibrator の最適化アルゴリズム

- **Context**: プレースホルダー実装を本番品質に置換
- **Alternatives Considered**:
  1. levenberg-marquardt クレートを直接使用
  2. pricer_core の汎用ソルバーを拡張
  3. 簡易 Newton-Raphson 実装
- **Selected Approach**: Option 1 - levenberg-marquardt クレートを使用
- **Rationale**:
  - 既に workspace に依存関係が存在
  - LM アルゴリズムは非線形最小二乗問題に最適
  - 十分にテストされた実装
- **Trade-offs**:
  - (+) 信頼性の高い実装、収束特性が良好
  - (-) 外部依存、パラメータ境界は手動で処理
- **Follow-up**: パラメータ境界制約のクリッピング実装を検証

### Decision: FxVolBuilder の拡張方式

- **Context**: FxVolInstrument を直接受け取れるようにする方法
- **Alternatives Considered**:
  1. FxVolBuilder に with_fx_curve, with_convention, add_instrument を追加
  2. FxVolInstrumentAdapter を新規作成して変換を担当
- **Selected Approach**: Option 1 - FxVolBuilder を直接拡張
- **Rationale**:
  - 既存の FxVolBuilder 構造が適切な拡張ポイントを提供
  - アダプター層は不要な複雑性を追加
  - infra_domain の型を直接受け取ることでAPI使用性が向上
- **Trade-offs**:
  - (+) シンプルなAPI、型安全
  - (-) FxVolBuilder の責務が増加
- **Follow-up**: FxVolBuilder が肥大化しないよう、内部ヘルパーを適切に分離

### Decision: FxCurve の型格納方式（FxCurveEnum）

- **Context**: FxVolBuilder が FxCurve を保持する際の型消去 vs 静的ディスパッチの選択
- **Alternatives Considered**:
  1. `Box<dyn FxCurve<T>>` — 動的ディスパッチ、型消去
  2. `FxCurveEnum<T>` — 静的ディスパッチ、enum によるバリアント管理
  3. ジェネリクス `FxVolBuilder<T, C: FxCurve<T>>` — 型パラメータ伝播
- **Selected Approach**: Option 2 - FxCurveEnum による静的ディスパッチ
- **Rationale**:
  - Enzyme AAD との互換性を維持（動的ディスパッチは Enzyme 非対応）
  - 既存の CurveEnum パターンに準拠（YieldCurve の先例あり）
  - ランタイムオーバーヘッドなし
  - 型消去による情報損失を回避
- **Trade-offs**:
  - (+) Enzyme 互換性、パフォーマンス、型安全性
  - (-) 新しい YieldCurve 実装追加時に FxCurveEnum への追加が必要
- **Follow-up**: IrpFlat, IrpBootstrapped, Irp の3バリアントで実用上十分か検証

---

## Risks & Mitigations

- **PremiumAdjusted delta 計算の複雑さ** — 反復ソルバーを使用し、収束条件を明確に定義
- **SABR キャリブレーション収束失敗** — 初期推定値の改善（ATM vol ベース）、パラメータ境界の適用
- **demo_gui リグレッション** — 段階的移行、既存テストの維持と並行実行
- **Float ジェネリクスの伝播** — 全ての新規関数で `<T: Float>` を一貫して使用

---

## References

- [Garman-Kohlhagen Model](pricer_core/src/math/formulas/garman_kohlhagen.rs) — 既存のFXオプションプライシング実装
- [SABR Model](pricer_core/src/math/formulas/sabr.rs) — Hagan公式によるインプライドボラティリティ計算
- [levenberg-marquardt crate](https://docs.rs/levenberg-marquardt) — 非線形最小二乗問題のソルバー
- [FX Delta Conventions](https://en.wikipedia.org/wiki/Foreign_exchange_derivative) — FXオプションのデルタ慣行
