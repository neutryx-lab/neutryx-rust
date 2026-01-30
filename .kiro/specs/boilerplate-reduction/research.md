# Gap Analysis: boilerplate-reduction

_作成日: 2026-01-30_

## 1. 現状調査

### 1.1 発見されたBuilder構造体

コードベース全体で **31個の手書き Builder 構造体** が確認されました。

#### infra_master (高優先度)

| ファイル | Builder | 行数 | 複雑度 |
|---------|---------|------|--------|
| `trade/builder.rs` | `LegBuilder`, `TradeBuilder` | 354行 | 中（バリデーション有り） |
| `counterparty/counterparty_entity.rs` | `CounterPartyBuilder` | 326行 | 低 |
| `counterparty/csa.rs` | `CsaTermsBuilder` | 576行 | 中（デフォルト値ロジック） |
| `counterparty/netting_set.rs` | `NettingSetBuilder`, `CrossBookNettingAgreementBuilder` | 1280行 | 高 |
| `counterparty/netting_agreement.rs` | `NettingAgreementBuilder` | 858行 | 中 |
| `counterparty/counterparty_portfolio.rs` | `VariationMarginAgreementBuilder`, `IsdaMasterAgreementBuilder`, `ExposurePathBuilder`, `CounterpartyPortfolioBuilder` | 1282行 | 高 |
| `book/book.rs` | `BookBuilder` | 391行 | 低 |
| `portfolio/portfolio.rs` | `PortfolioBuilder` | 570行 | 低 |
| `trade/instrument_def/fx_vol.rs` | `FxVolInstrumentBuilder` | - | 低 |

#### pricer_core (中優先度)

| ファイル | Builder | 複雑度 |
|---------|---------|--------|
| `kernel/pricing_kernel.rs` | `PricingKernelBuilder` | 高（SoA構造） |
| `kernel/script_kernel.rs` | `ScriptKernelBuilder` | 高 |
| `kernel/callable_kernel.rs` | `CallableKernelBuilder` | 高 |

#### pricer_models (中優先度)

| ファイル | Builder | 複雑度 |
|---------|---------|--------|
| `builder/matrix.rs` | `CalibrationMatrixBuilder<T>` | 高（ジェネリック） |
| `builder/vol/cube.rs` | `VolCubeBuilder<T>` | 高（ジェネリック） |
| `builder/vol/surface.rs` | `FxVolBuilder<T>` | 高（ジェネリック） |

#### pricer_pricing (中優先度)

| ファイル | Builder | 複雑度 |
|---------|---------|--------|
| `generic_pricer/config.rs` | `GreeksConfigBuilder`, `ModelConfigBuilder`, `PricerConfigBuilder` | 低〜中 |
| `methods/mc/config.rs` | `MonteCarloConfigBuilder` | 低 |
| `methods/tree/config.rs` | `TreeConfigBuilder` | 低 |
| `kernel/provider.rs` | `IndexedMarketAdapterBuilder` | 中 |
| `graph/extractor.rs` | `GraphBuilder` | 低 |

#### pricer_risk (低優先度)

| ファイル | Builder | 複雑度 |
|---------|---------|--------|
| `portfolio/trade.rs` | `TradeBuilder` | 低 |
| `portfolio/builder.rs` | `PortfolioBuilder` | 低 |
| `portfolio/sample_builder.rs` | `SamplePortfolioBuilder` | 低 |
| `greeks/config.rs` | `GreeksConfigBuilder` | 低 |

### 1.2 既存の命名規則・パターン

**共通パターン**:
- `StructName` + `Builder` という命名規則
- `fn new(required_fields) -> Self` コンストラクタ
- `fn field_name(mut self, value) -> Self` チェーンメソッド
- `fn build(self) -> TargetStruct` 終端メソッド
- `#[must_use]` 属性の使用
- `impl Into<T>` を活用した ergonomic API

**バリデーション例** (`LegBuilder::new`):
```rust
if schedule.len() < 2 {
    return Err(TradeError::InvalidSchedule(...));
}
if notional < 0.0 {
    return Err(TradeError::InvalidNotional(notional));
}
```

**デフォルト値ロジック例** (`CsaTermsBuilder::build`):
```rust
mpor_days: if self.mpor_days == 0 { 10 } else { self.mpor_days },
margin_currency: self.margin_currency.unwrap_or(Currency::USD),
```

### 1.3 依存関係・制約

- **strum**: 既にワークスペース依存関係に存在（v0.26）
- **bon**: 未使用（新規追加が必要）
- **derive_builder / typed-builder**: 未使用
- **A-I-P-S依存ルール**: Infra → Pricer の順で移行が必須

---

## 2. 要件実現可能性分析

### 2.1 技術要件マッピング

| 要件 | 現状 | ギャップ | 対応方針 |
|------|------|---------|---------|
| Req 1: bon依存関係追加 | 未導入 | bon クレート追加 | workspace.dependencies に追加 |
| Req 2: Trade関連Builder移行 | 手書き実装 | 約350行削減可能 | `#[derive(bon::Builder)]` 適用 |
| Req 3: 属性カスタマイズ | 既存API互換性要 | `#[builder(into)]`, `#[builder(default)]` | 要検証 |
| Req 4: テスト互換性 | 16テスト存在 | API変更時の更新 | 呼び出し側修正 |
| Req 5: ドキュメント | 既存コメント有り | steering更新 | dependency-management.md 更新 |
| Req 6: 拡張対象特定 | 31 Builder発見 | 優先順位付け | 本ドキュメントで完了 |

### 2.2 bon 互換性課題

#### 課題1: バリデーション付きコンストラクタ

`LegBuilder::new()` は `Result<Self, TradeError>` を返すため、bon の標準パターンと異なります。

**対応策**:
- `#[builder(with = |...| { validate(...)?; Ok(value) })]` で検証ロジック埋め込み
- または `build()` 後に `validate()` を呼び出す設計に変更

#### 課題2: 特殊な build メソッド

`LegBuilder` は `build_fixed(rate)` と `build_floating(index, spread)` の2つの終端メソッドを持ちます。

**対応策**:
- bon の `#[builder(finish_fn = ...)]` 機能の調査が必要
- または、`build()` 後に別メソッドで Leg 生成する設計に変更

#### 課題3: ジェネリック型 Builder

`CalibrationMatrixBuilder<T: Float>` 等はジェネリック型パラメータを持ちます。

**Research Needed**: bon のジェネリック構造体サポート状況

### 2.3 削減見積もり

| 対象 | 現在行数 | 削減後推定 | 削減率 |
|------|---------|-----------|--------|
| infra_master builders | 約5,637行 | 約1,500行 | 約73% |
| 手書きBuilder impl | 約3,000行 | 0行 | 100% |
| **合計推定削減** | **約3,000行** | - | - |

---

## 3. 実装アプローチオプション

### Option A: 段階的移行（推奨）

**説明**: infra_master の単純な Builder から開始し、複雑度の高い構造へ段階的に拡張。

**フェーズ**:
1. `BookBuilder`, `PortfolioBuilder` (単純、デフォルト値のみ)
2. `CounterPartyBuilder`, `CsaTermsBuilder` (デフォルト値ロジック有り)
3. `TradeBuilder`, `LegBuilder` (バリデーション有り、特殊終端メソッド)
4. `counterparty_portfolio.rs` の複数 Builder (複雑)

**トレードオフ**:
- ✅ リスク最小化、段階的学習
- ✅ 各フェーズで API 互換性確認可能
- ❌ 完了まで時間がかかる

### Option B: infra_master 一括移行

**説明**: infra_master 内の全 Builder を一度に移行。

**トレードオフ**:
- ✅ 一貫性の確保
- ✅ 完了が早い
- ❌ 大規模変更によるリグレッションリスク
- ❌ 複雑な Builder で問題発生時のロールバック困難

### Option C: ハイブリッド（拡張 + 新規）

**説明**: 単純な Builder は bon 移行、複雑な Builder は既存維持。

**フェーズ**:
1. 単純な Builder（10行以下の impl）を bon 化
2. 複雑な Builder は既存維持、将来課題として残す

**トレードオフ**:
- ✅ 即効性のある削減
- ✅ 複雑なケースのリスク回避
- ❌ コードベースに2つのパターンが混在

---

## 4. 実装複雑度とリスク

### 工数見積もり

**Option A（推奨）**: **M（3-7日）**
- 理由: bon の学習曲線、段階的移行による確認作業、テスト更新

**Option B**: **M〜L（5-10日）**
- 理由: 一括変更、広範なテスト影響

**Option C**: **S（1-3日）**
- 理由: 単純な Builder のみ対象

### リスク評価

| リスク | レベル | 緩和策 |
|--------|--------|--------|
| bon API 互換性 | 中 | 事前に PoC 実装で検証 |
| 既存テスト破損 | 低〜中 | 段階的移行で影響範囲限定 |
| 特殊パターン対応 | 中 | Research Needed として設計フェーズで調査 |
| ジェネリック Builder | 不明 | 設計フェーズで bon ドキュメント精査 |

---

## 5. Research Needed（設計フェーズへ持ち越し）

1. **bon のジェネリック構造体サポート**: `CalibrationMatrixBuilder<T: Float>` 等への適用可否
2. **カスタム finish_fn**: `build_fixed()` / `build_floating()` のような複数終端メソッドの実現方法
3. **バリデーション統合**: `#[builder(with = ...)]` でのエラー返却パターン
4. **bon バージョン選定**: 最新安定版の確認（crates.io）

---

## 6. 設計フェーズへの推奨事項

### 推奨アプローチ

**Option A: 段階的移行** を推奨。

### 推奨フェーズ順序

1. **Phase 1**: `BookBuilder`, `PortfolioBuilder`（単純、リスク低）
2. **Phase 2**: `CounterPartyBuilder`, `CsaTermsBuilder`（デフォルト値有り）
3. **Phase 3**: `TradeBuilder`, `LegBuilder`（バリデーション、特殊メソッド）
4. **Phase 4**: `pricer_*` クレートへの展開（A-I-P-S順守）

### 設計フェーズでの追加調査項目

- bon 公式ドキュメントの精査（`#[builder(with = ...)]`, `#[builder(finish_fn = ...)]`）
- 既存テストケースのAPI呼び出しパターン分析
- 呼び出し側（テスト以外）のインパクト調査

---

## 7. 結論

既存コードベースには31個の手書き Builder が存在し、約3,000行のボイラープレート削減ポテンシャルがあります。bon クレートの導入は技術的に実現可能ですが、いくつかの特殊パターン（バリデーション付きコンストラクタ、複数終端メソッド、ジェネリック型）については設計フェーズでの追加調査が必要です。

段階的移行アプローチにより、リスクを最小化しながら確実にボイラープレート削減を実現できます。
