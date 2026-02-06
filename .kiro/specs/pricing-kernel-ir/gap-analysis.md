# Gap Analysis: pricing-kernel-ir

## 概要

本ドキュメントは、PricingKernel IR（中間表現）機能の実装に向けて、既存コードベースとの差分および統合戦略を分析します。

---

## 1. 現状調査（Current State Investigation）

### 1.1 関連資産の特定

#### Trade階層構造（infra_domain）

| ファイル | 役割 | 関連度 |
|---------|------|-------|
| [crates/infra_domain/src/trade/trade.rs](crates/infra_domain/src/trade/trade.rs) | Trade定義（ID, legs, trade_type, metadata） | ⭐⭐⭐ 高 |
| [crates/infra_domain/src/trade/leg.rs](crates/infra_domain/src/trade/leg.rs) | Leg定義（cashflows, direction, leg_type, currency） | ⭐⭐⭐ 高 |
| [crates/infra_domain/src/trade/cashflow.rs](crates/infra_domain/src/trade/cashflow.rs) | Cashflow定義（payment_date, year_fraction, notional, payoff） | ⭐⭐⭐ 高 |

**観察**: 階層構造は十分に整理されており、IRへのコンパイルの入力として適切。`Cashflow`には`payment_date`, `year_fraction`, `notional`, `currency`など必要なフィールドが既に存在。

#### 既存SoAパターン（pricer_risk）

| ファイル | 役割 | 関連度 |
|---------|------|-------|
| [crates/pricer_risk/src/soa/trade_soa.rs](crates/pricer_risk/src/soa/trade_soa.rs) | オプション向けSoA（strikes, maturities, notionals, payoff_signs） | ⭐⭐⭐ 高 |
| [crates/pricer_risk/src/soa/exposure_soa.rs](crates/pricer_risk/src/soa/exposure_soa.rs) | エクスポージャー向けSoA | ⭐⭐ 中 |

**観察**: `TradeSoA`は既にSoAパターンを実装。`from_trades()`メソッドでAoS→SoA変換を行っている。この設計パターンを`PricingKernel`に拡張可能。

#### IndexedMarketパターン（pricer_models）

| ファイル | 役割 | 関連度 |
|---------|------|-------|
| [crates/pricer_models/src/market/indexed_market.rs](crates/pricer_models/src/market/indexed_market.rs) | インデックスキーによる市場データアクセス | ⭐⭐⭐ 高 |
| [crates/infra_domain/src/trade/index_requirement.rs](crates/infra_domain/src/trade/index_requirement.rs) | TradeIndexRequirements | ⭐⭐ 中 |

**観察**: `IndexedMarket<T>`はRateIndex/CurrencyPairをキーとしたHashMapベースのアクセスを提供。`PricingKernel`の`fwd_index_ids`と連携可能。

#### 3-Stage Rocketパターン（pricer_pricing）

| ファイル | 役割 | 関連度 |
|---------|------|-------|
| [crates/pricer_pricing/src/context.rs](crates/pricer_pricing/src/context.rs) | PricingContext（Stage 2）、price_single_trade（Stage 3） | ⭐⭐⭐ 高 |

**観察**: 既存の3-stage rocketパターン：
1. **Stage 1 (Definition)**: ModelEnum, InstrumentEnum
2. **Stage 2 (Linking)**: PricingContext（参照バインディング）
3. **Stage 3 (Execution)**: price_single_trade（純粋計算）

`PricingKernel`は新しいStage 2の表現として位置づけ可能。

#### Enzyme AD統合（pricer_risk）

| ファイル | 役割 | 関連度 |
|---------|------|-------|
| [crates/pricer_risk/src/enzyme/greeks.rs](crates/pricer_risk/src/enzyme/greeks.rs) | GreeksEnzyme, EnzymeGreeksResult | ⭐⭐⭐ 高 |
| [crates/pricer_risk/src/enzyme/forward.rs](crates/pricer_risk/src/enzyme/forward.rs) | Forward-mode AD | ⭐⭐ 中 |
| [crates/pricer_risk/src/enzyme/reverse.rs](crates/pricer_risk/src/enzyme/reverse.rs) | Reverse-mode AD | ⭐⭐ 中 |
| [crates/pricer_risk/src/enzyme/smooth.rs](crates/pricer_risk/src/enzyme/smooth.rs) | 不連続関数のSmooth近似 | ⭐⭐ 中 |

**観察**: Enzyme ADは`#[autodiff]`属性と共に使用。`smooth.rs`は`max(0, x)`などの不連続ペイオフをAD互換に変換。

### 1.2 コンベンション分析

| カテゴリ | 現在のパターン | IR設計への影響 |
|---------|--------------|---------------|
| 命名規則 | snake_case、British English（optimiser, serialisation） | `PricingKernel`, `TradeCompiler`で統一 |
| レイヤー依存 | A-I-P-S一方向フロー | IRは`pricer_core` (L1)に配置 |
| テスト配置 | 各モジュールの`tests`サブディレクトリ | 同様のパターンを踏襲 |
| エラー型 | カスタムエラー列挙型（PricingError, MarketDataError） | `CompileError`を新規定義 |

---

## 2. 要件実現性分析（Requirements Feasibility）

### 2.1 要件別ギャップマップ

| 要件ID | 技術ニーズ | 既存資産 | ギャップ | ステータス |
|--------|----------|---------|---------|-----------|
| **Req 1** | PricingKernel SoA構造体 | TradeSoA（参考パターン） | 新規作成必要 | Missing |
| **Req 2** | TradeCompilerトレイト | なし | 新規作成必要 | Missing |
| **Req 3** | IRS/Bond/FRAコンパイル | Trade/Leg/Cashflow定義あり | コンパイラロジック必要 | Missing |
| **Req 4** | X-Ccy/FX対応 | CurrencyPair, FxCurve | fx_index_ids拡張必要 | Partial |
| **Req 5** | CMS凸性調整 | VolCube | IndexedMarket連携 | Partial |
| **Req 6** | ScriptKernel（Path-Dep） | なし | 新規設計必要 | Missing |
| **Req 7** | CallableKernel（Bermudan） | なし | 新規設計必要 | Missing |
| **Req 8** | price_kernel関数 | price_single_trade（参考） | 新規実装必要 | Missing |
| **Req 9** | Date/Time分離 | Date型（i32 epoch） | 変換ロジック必要 | Partial |
| **Req 10** | A-I-P-S適合 | 明確なレイヤー分離 | 配置調整のみ | Low Risk |
| **Req 11** | SIMD最適化 | Rayon並列化 | アラインメント追加 | Constraint |
| **Req 12** | Enzyme AD互換 | enzyme moduleあり | 制御フロー排除 | Constraint |

### 2.2 複雑性シグナル

| 領域 | 複雑性タイプ | 説明 |
|------|------------|------|
| TradeCompiler | アルゴリズム | スケジュール展開、休日調整、YearFraction計算 |
| ScriptKernel | ワークフロー | イベント駆動実行、状態遷移管理 |
| CallableKernel | アルゴリズム | LSMC、Backward Induction、回帰分析 |
| IndexedMarket連携 | 統合 | 型パラメータ`T: Float`との整合性 |

---

## 3. 実装アプローチ選択肢

### Option A: 既存コンポーネント拡張

**対象**: `pricer_risk/src/soa`の拡張

**変更ファイル**:
- `crates/pricer_risk/src/soa/pricing_kernel.rs` (新規)
- `crates/pricer_risk/src/soa/mod.rs` (拡張)

**トレードオフ**:
- ✅ 既存SoAパターンの再利用
- ✅ テストインフラ活用可能
- ❌ `pricer_risk` (L4)への配置はA-I-P-S違反
- ❌ `pricer_core`からの依存関係逆転

**評価**: **不推奨** - レイヤー違反が発生

---

### Option B: 新規コンポーネント作成（推奨）

**対象**: `pricer_core`に新規モジュール作成

**新規ファイル構成**:
```
crates/pricer_core/src/
├── ir/
│   ├── mod.rs
│   ├── pricing_kernel.rs    # Req 1: PricingKernel構造体
│   ├── script_kernel.rs     # Req 6: ScriptKernel構造体
│   ├── callable_kernel.rs   # Req 7: CallableKernel構造体
│   └── error.rs             # CompileError定義
│
crates/pricer_models/src/
├── compiler/
│   ├── mod.rs
│   ├── trade_compiler.rs    # Req 2: TradeCompilerトレイト
│   ├── linear_products.rs   # Req 3: IRS/Bond/FRAコンパイラ
│   ├── xccy_compiler.rs     # Req 4: X-Ccy対応
│   └── exotic_compiler.rs   # Req 6-7: エキゾチック対応
│
crates/pricer_pricing/src/
├── kernel/
│   ├── mod.rs
│   ├── linear_engine.rs     # Req 8: price_kernel関数
│   └── script_engine.rs     # Req 6: ScriptKernel実行
```

**トレードオフ**:
- ✅ A-I-P-Sレイヤー分離を完全遵守
- ✅ 明確な責任境界
- ✅ 独立したテスト可能
- ❌ 新規ファイル数が多い
- ❌ 既存パターンとの統合設計が必要

**評価**: **推奨** - アーキテクチャ整合性が最優先

---

### Option C: ハイブリッドアプローチ

**対象**: 段階的導入

**フェーズ1（MVP）**:
- `pricer_core/src/ir/pricing_kernel.rs` のみ作成
- `TradeSoA`のパターンを参考に基本SoA構造を定義
- IRS/Bondのみ対応

**フェーズ2（拡張）**:
- ScriptKernel、CallableKernelを追加
- X-Ccy、CMSコンパイラ追加

**フェーズ3（最適化）**:
- SIMD最適化（64バイトアラインメント）
- Enzyme AD完全統合

**トレードオフ**:
- ✅ 段階的検証可能
- ✅ リスク分散
- ✅ 早期フィードバック取得
- ❌ 複数フェーズの調整が必要
- ❌ 中間状態での機能不完全

**評価**: **代替案** - 時間制約がある場合に有効

---

## 4. 技術的制約と未解決事項

### 4.1 アーキテクチャ制約

| 制約 | 影響 | 対応策 |
|------|-----|-------|
| A-I-P-S依存方向 | PricingKernelはpricer_coreに配置必須 | Option Bを採用 |
| IndexedMarket<T>のジェネリクス | price_kernelも`T: Float`が必要 | 型パラメータ伝播 |
| Enzyme ADの制御フロー制限 | match/ifはAD不可 | 配列インデックスアクセスのみ |

### 4.2 Research Needed（設計フェーズで調査）

| 項目 | 詳細 | 優先度 |
|------|-----|-------|
| **SIMD アラインメント** | `Vec<f64>`の64バイトアラインメント方法（`aligned_vec`クレート？） | 高 |
| **Enzyme互換型** | `Vec<i32>`（日付）がEnzyme微分を通過するか検証 | 高 |
| **LSMC回帰** | Backward Passの回帰多項式実装（既存ライブラリ vs 自前実装） | 中 |
| **CMS凸性調整** | VolCubeからの凸性調整計算式の確定 | 中 |

---

## 5. 実装複雑性とリスク評価

### 5.1 工数見積もり

| コンポーネント | 工数 | 根拠 |
|--------------|------|-----|
| PricingKernel構造体 (Req 1) | **S** | 既存SoAパターンの適用、単純なデータ構造 |
| TradeCompiler (Req 2-3) | **M** | スケジュール展開ロジック、複数商品対応 |
| X-Ccy/CMS対応 (Req 4-5) | **M** | IndexedMarket連携、凸性調整計算 |
| ScriptKernel (Req 6) | **L** | 新規イベント駆動設計、状態管理 |
| CallableKernel (Req 7) | **XL** | LSMC実装、Backward Induction |
| price_kernel関数 (Req 8) | **S** | 既存price_single_tradeの参考あり |
| SIMD/Enzyme最適化 (Req 11-12) | **M** | アラインメント、smooth関数適用 |

**総合工数**: **L～XL**（2週間以上）

### 5.2 リスク評価

| リスク | レベル | 根拠 |
|-------|-------|-----|
| **技術リスク** | Medium | Enzyme ADとの統合は既存実績あり |
| **統合リスク** | Medium | IndexedMarket、3-stage rocketとの連携設計必要 |
| **スコープリスク** | High | CallableKernel/LSMCは複雑性が高い |

---

## 6. 推奨事項

### 設計フェーズへの推奨

1. **アプローチ選択**: **Option B（新規コンポーネント作成）** を推奨
   - A-I-P-Sアーキテクチャ整合性を最優先
   - 長期的な保守性を確保

2. **段階的実装**: Option Cのフェーズ分けを設計に組み込む
   - Phase 1: PricingKernel + LinearProductsCompiler
   - Phase 2: ScriptKernel + ExoticCompiler
   - Phase 3: CallableKernel + LSMCEngine

3. **Research Items**: 設計フェーズで以下を調査
   - SIMDアラインメントの実装方法
   - Enzyme ADとVec<i32>の互換性
   - 既存LSMCライブラリの評価

4. **優先順位**: Req 7（CallableKernel）は複雑性が高いため、設計で明確なスコープ定義が必要

---

## 7. 次のステップ

**Gap Analysisの結論**: 実装は実現可能だが、複雑性が高い要件（Req 6-7）には追加調査が必要。

**推奨アクション**:
```
/kiro:spec-design pricing-kernel-ir
```

設計フェーズで以下を決定:
- 詳細なモジュール構成とインターフェース設計
- IndexedMarket<T>との型パラメータ戦略
- LSMC実装の詳細仕様
