# ギャップ分析: move-enzyme-to-pricer-risk

## 1. 現状調査

### 1.1 enzymeモジュール構成

**場所**: `crates/pricer_pricing/src/enzyme/`

| ファイル | 目的 | 行数 |
|---------|------|-----|
| `mod.rs` | エントリポイント、ADMode, Activity, gradient関数 | ~550 |
| `forward.rs` | フォワードモードAD型 (ForwardAD<T>) | ~400 |
| `reverse.rs` | リバースモードAD型 (ReverseAD<T>, GammaAD<T>) | ~300 |
| `greeks.rs` | Greeks計算トレイト・実装 | ~900 |
| `loops.rs` | Enzyme互換ループパターン | ~650 |
| `parallel.rs` | 並列adjoint集約 | ~400 |
| `smooth.rs` | スムース近似関数 | ~400 |
| `checkpoint_ad.rs` | チェックポイントAD | ~400 |
| `fallback.rs` | 有限差分フォールバック | ~350 |
| `verification.rs` | 検証ユーティリティ | ~500 |
| `wrappers.rs` | Enzyme `#[autodiff_*]`マクロラッパー | ~500 |

**合計**: 約5,350行

### 1.2 依存関係

**pricer_pricing/Cargo.toml（抜粋）**:
```toml
[dependencies]
llvm-sys = { version = "180", features = ["prefer-dynamic"], optional = true }

[features]
enzyme-ad = ["dep:llvm-sys"]
```

**pricer_risk/Cargo.toml（抜粋）**:
```toml
[dependencies]
pricer_pricing = { path = "../pricer_pricing", features = ["l1l2-integration"] }

[features]
enzyme-ad = []  # 空のフィーチャー（実装なし）
```

### 1.3 enzymeの内部参照

| 参照元 | 参照内容 | 種別 |
|--------|---------|------|
| `pricer_pricing/src/lib.rs:159` | `pub use enzyme::{gradient, gradient_with_step, ADMode, Activity}` | re-export |
| `pricer_pricing/src/verify_enzyme.rs:24` | `enzyme::gradient` | テスト |
| enzyme内サブモジュール | `super::`による相互参照 | 内部 |

### 1.4 外部参照（コード内）

**実コードでの参照**: **なし**
- `pricer_pricing::enzyme`への外部参照はすべてdocstring/コメント内
- 実際のuseステートメントでの外部クレートからの参照は確認されず

### 1.5 アーキテクチャ上の位置づけ

```
現状（structure.md記載）:
┌─────────────────────────────────────────────────┐
│ L4: pricer_risk (Stable)                        │
│   exposure/, xva/, scenarios/, parallel/        │
│   ↓ depends on                                  │
├─────────────────────────────────────────────────┤
│ L3: pricer_pricing (Nightly + Enzyme)           │
│   enzyme/, mc/, rng/, path_dependent/, graph/   │
│   ↓ depends on                                  │
├─────────────────────────────────────────────────┤
│ L2: pricer_models (Stable)                      │
│ L1: pricer_core (Stable)                        │
└─────────────────────────────────────────────────┘
```

## 2. 要件実現可能性分析

### 2.1 技術要件マッピング

| 要件 | 必要アセット | 現状 | ギャップ |
|------|-------------|------|---------|
| R1: モジュール移動 | ファイルシステム操作 | 11ファイル存在 | なし |
| R2: 依存関係更新 | Cargo.toml変更 | llvm-sys依存あり | pricer_riskにllvm-sys追加必要 |
| R3: 参照更新 | パス変更 | docstring多数 | docstring全更新必要 |
| R4: ドキュメント更新 | steering/*.md | 現状記載あり | 2ファイル更新必要 |
| R5: ビルド検証 | cargo build/test | 既存CIあり | nightlyビルド設定追加 |

### 2.2 制約事項

#### アーキテクチャ制約（Critical）

**問題**: 現在のA-I-P-S依存ルールに抵触する可能性

```
移動後の依存関係:
pricer_risk (L4 + enzyme) ← pricer_pricing (L3) が必要になる場合
                         ↑
                         循環依存の可能性
```

**理由**:
1. `pricer_pricing/src/verify_enzyme.rs`がenzymeを使用
2. 移動後、pricer_pricingがenzymeを使うにはpricer_riskに依存が必要
3. しかしpricer_riskはpricer_pricingに依存（循環）

#### Nightly Rust制約

- **現状**: pricer_pricingのみnightly必須、pricer_riskはstable
- **移動後**: pricer_riskがnightly必須に変更
- **影響**: `#![feature(autodiff)]`がpricer_riskに必要

### 2.3 Unknown / Research Needed

| 項目 | 内容 | 優先度 |
|------|------|--------|
| verify_enzyme.rs | 移動後の配置先検討（pricer_riskに移動 or 削除） | P0 |
| greeks.rs依存 | `crate::mc`への依存をどう解決するか | P0 |
| lib.rs re-export | 移動後のpricer_pricingでのAPI維持方法 | P1 |

## 3. 実装アプローチオプション

### Option A: 完全移動（要件通り）

**概要**: enzyme全体をpricer_riskに移動、pricer_pricingからの参照を削除

**変更内容**:
1. `crates/pricer_pricing/src/enzyme/` → `crates/pricer_risk/src/enzyme/`
2. pricer_risk/Cargo.tomlにllvm-sys依存追加
3. pricer_risk/lib.rsにenzymeモジュール追加
4. pricer_pricing/src/verify_enzyme.rsをpricer_risk/src/tests/に移動
5. enzyme/greeks.rsの`crate::mc`依存を削除または抽象化
6. 全docstringのパス更新

**トレードオフ**:
- ✅ 要件に完全準拠
- ✅ リスク計算とAADが同一クレートに
- ❌ **greeks.rsの`MonteCarloPricer`依存が問題**（MCはpricer_pricingに残る）
- ❌ pricer_riskがnightly Rust必須に
- ❌ アーキテクチャドキュメント大幅改訂必要

### Option B: コア機能のみ移動（部分移動）

**概要**: AADコア（forward.rs, reverse.rs, smooth.rs, loops.rs）のみ移動、MC連携部分は残留

**変更内容**:
1. `enzyme/forward.rs`, `reverse.rs`, `smooth.rs`, `loops.rs` → pricer_risk
2. `enzyme/greeks.rs`, `verification.rs` → pricer_pricingに残留
3. pricer_riskにenzyme_core/を新設

**トレードオフ**:
- ✅ MonteCarlo依存問題を回避
- ✅ 段階的移行が可能
- ❌ enzymeが2クレートに分散
- ❌ 保守性低下（どこに何があるか不明瞭）

### Option C: 依存逆転（Hybrid）

**概要**: enzymeを独立クレート化、L3/L4両方から依存

**変更内容**:
1. 新規クレート `pricer_enzyme` (L2.5相当) を作成
2. pricer_pricing, pricer_riskの両方から依存
3. MC連携はpricer_pricingに残る

**トレードオフ**:
- ✅ 依存関係がクリーン
- ✅ enzymeの独立性確保
- ❌ 新規クレート追加（メンテナンスコスト増）
- ❌ 要件の「pricer_riskに移動」から逸脱

## 4. 複雑性・リスク評価

### 工数見積もり

| タスク | 工数 |
|--------|------|
| ファイル移動 | S (数時間) |
| Cargo.toml更新 | S (数時間) |
| greeks.rs依存解決 | **M-L (1-2週間)** |
| docstring更新 | S (1日) |
| steering更新 | S (数時間) |
| テスト修正 | M (数日) |
| **合計** | **M-L (1-2週間)** |

### リスク評価: **Medium-High**

| リスク要因 | 説明 |
|-----------|------|
| greeks.rs依存 | MonteCarloPricerへの直接依存が最大の課題 |
| アーキテクチャ変更 | L3/L4の役割定義変更を伴う |
| Nightly伝播 | pricer_riskがnightly必須化 |

## 5. 設計フェーズへの推奨事項

### 推奨アプローチ: **Option Aを基本とし、greeks.rs依存を設計で解決**

**理由**:
1. 要件の意図（AADはリスク計算のため）に最も合致
2. Option B/Cは中途半端で長期的に混乱を招く

### 設計フェーズで解決すべき課題

1. **greeks.rs依存解決（P0）**:
   - `GreeksEnzyme`トレイトをenzymeモジュールに残し、`MonteCarloPricer`実装はpricer_pricingに残す
   - または、トレイトを汎化して`impl`をfeature-gateする

2. **verify_enzyme.rs配置（P0）**:
   - pricer_riskの統合テストに移動
   - または削除してpricer_riskでenzymeテストを再構築

3. **API互換性（P1）**:
   - pricer_pricingからpricer_risk::enzymeをre-exportするか検討
   - 移行期間中の後方互換性確保

### Research Items for Design Phase

- [ ] `MonteCarloPricer`とenzymeの疎結合化手法
- [ ] nightly Rustのpricer_risk適用影響範囲
- [ ] CI/CDパイプラインへの影響
