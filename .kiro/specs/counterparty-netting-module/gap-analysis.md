# Gap Analysis: counterparty-netting-module

## 1. Current State Investigation

### 1.1 既存のドメイン関連アセット

| ファイル/モジュール | 場所 | 説明 |
|---------------------|------|------|
| `counterparty.rs` | `crates/infra_master/src/` | `CsaTerms`, `NettingSetConfig`を定義（現行の実装） |
| `portfolio/ids.rs` | `crates/pricer_risk/src/` | `CounterpartyId`, `NettingSetId`, `TradeId`の新型パターン |
| `portfolio/netting_set.rs` | `crates/pricer_risk/src/` | `NettingSet`, `CollateralAgreement`のXVA計算用型 |
| `portfolio/trade.rs` | `crates/pricer_risk/src/` | `Trade`構造体（CounterpartyId/NettingSetIdを参照） |
| `csa.rs` | `crates/adapter_loader/src/` | `infra_master`からの再エクスポート |
| `counterparties.csv` | `demo/data/input/counterparties/` | デモ用取引相手先データ |
| `netting_sets.csv` | `demo/data/input/counterparties/` | デモ用ネッティングセットデータ |

### 1.2 既存パターン（time/trade/conventionモジュール参考）

`infra_master`には既にサブモジュールパターンが確立されている：

```text
infra_master/src/
├── time/           # 新設モジュール（calendars, day_counters, error, period, types）
├── trade/          # 新設モジュール（builder, cashflow, error, index, leg, payoff, trade）
├── convention/     # 新設モジュール（swap, fra, futures, capfloor, fx, cds, bond）
├── counterparty.rs # レガシー単一ファイル（移行対象）
└── lib.rs
```

### 1.3 命名・レイヤリング規約

- **British English**: `optimiser`, `serialisation`
- **新型パターン**: ID型は`pub struct XxxId(String)`で実装
- **ビルダーパターン**: `XxxBuilder::new().with_xxx().build()`
- **エラー型**: `thiserror`使用、`XxxError` enum
- **Feature gate**: `#[cfg_attr(feature = "serde", derive(...))]`
- **後方互換**: クレートルートからの再エクスポート維持

### 1.4 統合ポイント

| 依存元 | 使用型 | 備考 |
|--------|--------|------|
| `pricer_risk` | `CounterpartyId`, `NettingSetId` | 独自のID型を定義済み（重複あり） |
| `pricer_risk` | `CollateralAgreement` | CSA条件の拡張版（`infra_master::CsaTerms`とは別） |
| `adapter_loader` | `CsaTerms`, `NettingSetConfig` | 再エクスポートのみ |
| `demo/inputs` | CSVからの読み込み | `counterparty_id`, `rating`, `sector`等のフィールドあり |

---

## 2. Requirements Feasibility Analysis

### 2.1 技術的ニーズ（EARS要件から）

| 要件 | データモデル | API/サービス | バリデーション |
|------|--------------|--------------|----------------|
| Req 1: モジュール構造 | `mod.rs`、5ファイル構成 | 再エクスポート | 後方互換性 |
| Req 2: CounterParty型 | `CounterParty`, `CounterPartyId`, `CounterPartySector`, `CreditRating` | ビルダー | IDバリデーション |
| Req 3: NettingSet型 | `NettingSet`, `NettingSetId`, `NettingType`, `MarginType` | ビルダー | 必須フィールド検証 |
| Req 4: CSA条件拡張 | `EligibleCollateral`, `Haircut` | 既存互換 | 通貨別閾値 |
| Req 5: エラー型 | `CounterPartyError` | From実装 | thiserror |
| Req 6: 型安全ID | 新型パターン | Display, Hash等 | serde transparent |

### 2.2 ギャップと制約

| ギャップ | カテゴリ | 影響度 |
|----------|----------|--------|
| `pricer_risk`との型重複 | **Constraint** | 中 - ID型が両クレートで定義されている |
| CSA条件の二重定義 | **Constraint** | 中 - `infra_master::CsaTerms`と`pricer_risk::CollateralAgreement`が共存 |
| デモデータのフィールド | **Missing** | 低 - `lei`, `pd_1y`フィールドが要件にない |
| 格付けの詳細設計 | **Research Needed** | 低 - S&P/Moody's/Fitchの統一スキーム |

### 2.3 複雑性シグナル

- **CRUD型**: 基本的なデータ構造定義とビルダー
- **ワークフロー**: なし（静的マスターデータ）
- **外部統合**: なし（内部モジュール再編成）
- **アルゴリズム**: なし

---

## 3. Implementation Approach Options

### Option A: 既存コンポーネント拡張

**概要**: `counterparty.rs`を`counterparty/`フォルダに展開し、既存型を拡張

**変更対象ファイル**:
- `crates/infra_master/src/counterparty.rs` → 削除
- `crates/infra_master/src/counterparty/mod.rs` → 新規作成
- `crates/infra_master/src/counterparty/csa.rs` → `CsaTerms`移行・拡張
- `crates/infra_master/src/counterparty/netting_set.rs` → `NettingSetConfig`移行・拡張
- `crates/infra_master/src/counterparty/counterparty.rs` → `CounterParty`新規
- `crates/infra_master/src/counterparty/error.rs` → `CounterPartyError`新規
- `crates/infra_master/src/lib.rs` → 再エクスポート更新

**Trade-offs**:
- ✅ 後方互換性を完全に維持
- ✅ `time/`, `trade/`, `convention/`と同じパターン
- ✅ 最小限の外部影響
- ❌ `pricer_risk`との型重複は解決しない

### Option B: 新規コンポーネント作成 + 統一

**概要**: `infra_master`に新モジュールを作成し、`pricer_risk`のID型を`infra_master`からの再エクスポートに移行

**追加変更対象**:
- `crates/pricer_risk/src/portfolio/ids.rs` → `infra_master`からの再エクスポートに変更
- `crates/pricer_risk/src/portfolio/netting_set.rs` → `CollateralAgreement`を`infra_master::CsaTerms`ベースに統一

**Trade-offs**:
- ✅ 型の一元管理（Single Source of Truth）
- ✅ 長期的な保守性向上
- ❌ `pricer_risk`のAPIブレーク
- ❌ 影響範囲が大きい（デモ、テスト等）
- ❌ A-I-P-S依存規則に抵触の可能性（PricerがInfraに依存）

### Option C: ハイブリッドアプローチ（推奨）

**概要**:
- Phase 1: `infra_master`に`counterparty/`モジュールを新設（Option A相当）
- Phase 2: （将来）`pricer_risk`のID型を`infra_master`からのインポートに段階的移行

**実装戦略**:
1. `infra_master`に完全な`counterparty/`モジュールを構築
2. `CounterPartyId`/`NettingSetId`を新型パターンで定義
3. 既存の`CsaTerms`/`NettingSetConfig`は後方互換で維持
4. `pricer_risk`は**変更しない**（現時点では両方のID型が共存）

**Trade-offs**:
- ✅ 後方互換性を完全維持
- ✅ 段階的移行が可能
- ✅ 影響範囲を最小化
- ❌ 一時的な型重複（許容可能）

---

## 4. Implementation Complexity & Risk

### Effort: **S (1-3 days)**

**理由**:
- 既存パターン（`time/`, `trade/`, `convention/`）に完全に従う
- 新規ロジックなし、データ構造定義のみ
- 既存テストパターンを踏襲

### Risk: **Low**

**理由**:
- 確立されたパターンの拡張
- 外部依存なし
- 後方互換性を維持する設計
- `pricer_risk`との統合は将来課題として分離

---

## 5. Recommendations for Design Phase

### 推奨アプローチ
**Option C: ハイブリッドアプローチ**

### 設計フェーズでの決定事項

1. **ID型の内部表現**: `String` vs `Arc<str>` vs `SmolStr`
2. **CreditRating詳細**: S&P/Moody's/Fitch統一 or 汎用enum
3. **EligibleCollateralの拡張性**: 固定enum vs 拡張可能なスキーム
4. **通貨別閾値の実装**: `HashMap<Currency, f64>` vs 専用構造体

### Research Items（設計フェーズへ）

1. **格付けスキーム**: 業界標準の格付け体系（S&P基準で十分か）
2. **LEI/ISINの扱い**: 要件には明示されていないが、デモデータにはLEIが存在
3. **pd_1y（1年デフォルト確率）**: XVA計算で必要となる可能性

---

## 6. Requirement-to-Asset Map

| 要件 | 既存アセット | ギャップ |
|------|--------------|----------|
| Req 1: モジュール構造 | `counterparty.rs`（単一ファイル） | フォルダ構造化 |
| Req 2: CounterParty型 | なし | **Missing** - 完全新規 |
| Req 3: NettingSet型 | `NettingSetConfig`（基本） | 拡張（NettingType, MarginType） |
| Req 4: CSA条件拡張 | `CsaTerms`（基本） | 拡張（EligibleCollateral, Haircut） |
| Req 5: エラー型 | `MasterDataError`（汎用） | **Missing** - 専用エラー型 |
| Req 6: 型安全ID | なし（`String`フィールド） | **Missing** - 新型パターン |
