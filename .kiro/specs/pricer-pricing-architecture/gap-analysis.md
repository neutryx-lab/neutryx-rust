# Gap Analysis: pricer-pricing-architecture

## 1. Current State Investigation

### 1.1 既存アセットのスキャン

#### モジュール構造 (`crates/pricer_pricing/src/`)

| モジュール | 内容 | 状態 |
|----------|------|------|
| `generic_pricer/` | GenericPricer, PricingResult, Config | ✅ 存在 |
| `mc/` | MonteCarloPricer, GbmParams, workspace | ✅ 存在 |
| `enzyme/` | Enzyme AD bindings | ✅ 存在 |
| `rng/` | PRNG, QMC sequences | ✅ 存在 |
| `checkpoint/` | Memory checkpointing | ✅ 存在 |
| `path_dependent/` | Asian, Barrier, Lookback | ✅ 存在 |
| `analytical/` | Asian, Barrier closed-form | ✅ 存在 |
| `graph/` | Computation graph export | ✅ 存在 |
| `pool/` | Thread-local buffer pool | ✅ 存在 |

#### 主要コンポーネント

**GenericPricer** ([generic_pricer/pricer.rs](crates/pricer_pricing/src/generic_pricer/pricer.rs)):
- `MarketProvider` との統合（l1l2-integration feature）
- Trade/Leg/Cashflow 階層のPV計算
- PayoffEvaluator によるペイオフ評価
- Standalone mode（市場データなし）対応

**MonteCarloPricer** ([mc/pricer.rs](crates/pricer_pricing/src/mc/pricer.rs)):
- GBM パス生成
- 欧州型・パス依存型オプション
- Greeks計算（bump-and-revalue）
- Forward-mode AD prototype

**PricingConfig** ([infra_config/src/pricing_config.rs](crates/infra_config/src/pricing_config.rs)):
- `PricingMethod` enum: `Analytical`, `MonteCarlo`
- MonteCarloParams（num_paths, num_steps, seed）
- valuation_date, reporting_currency

**MarketProvider** ([pricer_models/src/market/provider.rs](crates/pricer_models/src/market/provider.rs)):
- Arc-cached yield curves
- Arc-cached vol surfaces
- VolCube lazy evaluator

### 1.2 コンベンション分析

| 項目 | パターン |
|------|---------|
| 命名規則 | snake_case（モジュール）, PascalCase（型） |
| エラー型 | thiserror派生, 各モジュール固有 |
| 設定 | Builder pattern, Default trait |
| Feature flags | `l1l2-integration`, `enzyme-ad` |
| テスト配置 | 同一ファイル内 `#[cfg(test)]` |

### 1.3 統合サーフェス

| 統合ポイント | 現状 |
|-------------|------|
| PricingConfig → GenericPricer | `from_config()` 存在 |
| MarketProvider → GenericPricer | `new(market, ...)` 存在 |
| Trade → GenericPricer | `get_pv(trade, ...)` 存在 |
| PricingInstrument → Pricer | 部分的（VanillaOption, Forward） |

---

## 2. 要件実現性分析

### 2.1 要件マッピング

| 要件ID | 技術要件 | 現状 | ギャップ |
|--------|---------|------|---------|
| R1 | Pricer抽象トレイト | GenericPricer（具象） | **Missing**: 統一トレイト |
| R2 | 設定駆動型 | PricingConfig存在 | **Missing**: Tree手法 |
| R3 | Discount手法 | generic_pricer/kernel.rs | ✅ 存在 |
| R4 | Monte Carlo手法 | mc/ module | ✅ 存在 |
| R5 | Tree手法 | なし | **Missing**: 新規実装 |
| R6 | PricingResult統一 | generic_pricer/result.rs | 要調整（MC統合） |
| R7 | pricer_models統合 | l1l2-integration | ✅ 存在 |
| R8 | PricingInstrument連携 | pricing_instrument.rs | 要拡張 |
| R9 | エラーハンドリング | PricingError存在 | 要バリアント追加 |
| R10 | モジュール構造 | 現行構造 | 要リファクタリング |

### 2.2 ギャップ詳細

#### **Critical Gap: Tree手法未実装**
- Binomial Tree, Trinomial Tree が存在しない
- American option pricing に必要
- **Research Needed**: Cox-Ross-Rubinstein vs Jarrow-Rudd

#### **Gap: 統一Pricerトレイト**
- `GenericPricer` と `MonteCarloPricer` が独立
- 統一インターフェースで手法選択が必要
- 現状: `PricingMethod` enum は Analytical/MonteCarlo のみ

#### **Gap: PricingResultの統合**
- `mc::PricingResult` と `generic_pricer::PricingResult` が別型
- Standard Error は MC のみ
- 統一が必要

#### **Gap: 商品定義拡張**
- `PricingInstrument` は VanillaOption, Forward のみ
- Swap, Swaption, Asian 等が未対応

### 2.3 複雑性シグナル

| 要素 | 複雑性 | 理由 |
|------|--------|------|
| Tree実装 | **High** | 新規アルゴリズム、AD統合 |
| Pricer統一 | **Medium** | 既存コード影響範囲 |
| PricingResult統合 | **Low** | 型変換・マッピング |
| Config拡張 | **Low** | enum追加 |

---

## 3. 実装アプローチオプション

### Option A: 既存コンポーネント拡張

**概要**: GenericPricer を中心に拡張

**変更ファイル**:
- [generic_pricer/pricer.rs](crates/pricer_pricing/src/generic_pricer/pricer.rs): 手法選択ロジック追加
- [generic_pricer/config.rs](crates/pricer_pricing/src/generic_pricer/config.rs): TreeConfig追加
- `infra_config/pricing_config.rs`: `PricingMethod::Tree` 追加

**トレードオフ**:
- ✅ 最小限の新規ファイル
- ✅ 既存テストの再利用
- ❌ GenericPricer の肥大化リスク
- ❌ MC と Discount の責務混在

### Option B: 新規コンポーネント作成

**概要**: 階層的Pricer構造を新規設計

**新規ファイル**:
```
pricer_pricing/src/
├── pricer/              # 新規: 中央Pricer
│   ├── mod.rs
│   ├── traits.rs        # PricingMethod trait
│   ├── dispatcher.rs    # 手法選択
│   └── unified.rs       # UnifiedPricer
├── methods/             # 新規: 手法別実装
│   ├── discount/
│   ├── mc/              # 既存mc/を移動
│   └── tree/            # 新規
├── config/              # 新規: 設定統合
└── result/              # 新規: 結果統合
```

**トレードオフ**:
- ✅ 明確な責務分離
- ✅ 将来の拡張性
- ❌ 大規模リファクタリング
- ❌ 既存コードの移動コスト

### Option C: ハイブリッドアプローチ（推奨）

**概要**: 既存を活かしつつ段階的に整理

**フェーズ1: 最小限の変更**
- `PricingMethod::Tree` 追加
- Tree モジュール新規作成 (`tree/`)
- `generic_pricer` に手法ディスパッチャ追加

**フェーズ2: 構造整理**
- PricingResult 統一
- Pricer trait 導入（optional）
- methods/ への整理

**変更ファイル**:

| ファイル | 変更内容 |
|----------|----------|
| `lib.rs` | `pub mod tree;` 追加 |
| `tree/mod.rs` | 新規: Binomial/Trinomial |
| `generic_pricer/config.rs` | TreeConfig 追加 |
| `generic_pricer/pricer.rs` | 手法ディスパッチャ |
| `infra_config/pricing_config.rs` | `PricingMethod::Tree` |

**トレードオフ**:
- ✅ 段階的移行可能
- ✅ 既存機能の互換性維持
- ✅ リスク分散
- ❌ 一時的な構造の不整合

---

## 4. 実装複雑性とリスク

### 工数見積

| コンポーネント | 工数 | 根拠 |
|--------------|------|------|
| Tree モジュール | **L** (1-2週間) | 新規アルゴリズム、テスト |
| Pricer dispatcher | **M** (3-7日) | 既存統合、パターン適用 |
| Config 拡張 | **S** (1-3日) | enum追加、バリデーション |
| PricingResult 統一 | **S** (1-3日) | 型変換、マッピング |
| エラー型拡張 | **S** (1-3日) | バリアント追加 |

**全体工数**: **L** (1-2週間)

### リスク評価

| リスク | レベル | 緩和策 |
|--------|--------|--------|
| Tree AD 統合 | **High** | bump-and-revalue 先行、Enzyme後回し |
| 既存API破壊 | **Medium** | feature flag で段階導入 |
| パフォーマンス | **Medium** | ベンチマーク早期導入 |
| テストカバレッジ | **Low** | TDD アプローチ |

---

## 5. 設計フェーズへの推奨事項

### 推奨アプローチ: Option C (ハイブリッド)

**理由**:
1. 既存 `generic_pricer` の成熟度が高い
2. Tree は独立モジュールとして追加可能
3. 段階的リファクタリングでリスク分散

### 設計フェーズでの調査項目

1. **Tree アルゴリズム選択**
   - Cox-Ross-Rubinstein (CRR) vs Jarrow-Rudd
   - 収束特性、精度、パフォーマンス

2. **AD 統合戦略**
   - Tree ノードでの tangent propagation
   - Enzyme compatibility

3. **商品拡張**
   - American option → Tree
   - Bermudan option → Tree
   - Asian → MC (既存)

### 優先順位

1. **Phase 1**: Tree モジュール実装（American option サポート）
2. **Phase 2**: GenericPricer 手法ディスパッチャ統合
3. **Phase 3**: PricingResult 統一、モジュール整理

---

## 6. Research Needed

| 項目 | 詳細 | 優先度 |
|------|------|--------|
| Tree AD | Binomial tree での AD 実装パターン | High |
| American Greeks | 早期行使境界での微分 | Medium |
| SABR Tree | 確率的ボラティリティ対応 | Low |

---

_Generated: 2026-01-26_
_Spec: pricer-pricing-architecture_
