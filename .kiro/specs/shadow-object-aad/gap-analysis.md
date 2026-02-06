# Gap Analysis: shadow-object-aad

## 1. 現状調査

### 1.1 関連ファイル・モジュール構造

#### pricer_risk::enzyme モジュール（L4）
| ファイル | 役割 | 関連度 |
|---------|------|--------|
| [mod.rs](crates/pricer_risk/src/enzyme/mod.rs) | ADMode, Activity 列挙型、gradient 関数 | ⭐⭐⭐ 統合先 |
| [wrappers.rs](crates/pricer_risk/src/enzyme/wrappers.rs) | `#[autodiff]` マクロラッパー、AllGreeks 構造体 | ⭐⭐⭐ パターン参照 |
| [greeks.rs](crates/pricer_risk/src/enzyme/greeks.rs) | GreeksEnzyme トレイト、EnzymeGreeksResult | ⭐⭐ 既存 API |
| [reverse.rs](crates/pricer_risk/src/enzyme/reverse.rs) | ReverseAD<T>, GammaAD<T>, CompleteGreeks | ⭐⭐ 結果型参照 |

#### マーケットデータ構造（pricer_models）
| 構造体 | 場所 | ジェネリクス | Vec<f64> フィールド |
|--------|------|--------------|-------------------|
| `InterpolatedCurve<T>` | [curves/interpolated.rs](crates/pricer_models/src/market/curves/interpolated.rs) | `T: Float` | `tenors`, `rates` |
| `CreditCurve<T>` | [curves/credit.rs](crates/pricer_models/src/market/curves/credit.rs) | `T: Float` | `tenors`, `hazard_rates` |
| `FlatCurve<T>` | [curves/flat.rs](crates/pricer_models/src/market/curves/flat.rs) | `T: Float` | なし（単一 `rate` 値）|

#### 既存カーネルパターン
| 構造体 | 場所 | 特徴 |
|--------|------|------|
| `PricingKernel` | [pricer_core/src/ir/pricing_kernel.rs](crates/pricer_core/src/ir/pricing_kernel.rs) | SoA レイアウト、`AlignedBuffer<f64>`、ジェネリクスなし |

### 1.2 既存規約と統合サーフェス

#### Activity 列挙型（既存）
```rust
pub enum Activity {
    Const,        // 定数（微分対象外）
    Dual,         // Forward mode tangent
    Active,       // Reverse mode scalar
    Duplicated,   // Reverse mode shadow buffer ← Requirement 3.6 で使用
    DuplicatedOnly,
}
```

#### 既存の `#[autodiff]` パターン
```rust
// wrappers.rs より
#[autodiff(d_price_all, Reverse, Duplicated, Const, Duplicated, Duplicated, Duplicated, Active)]
pub fn price_european_call_adjoint(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> f64
```

**観察**: 現在の Enzyme 統合は **スカラー引数**（f64）に対して行われている。**スライス引数**（`&[f64]`）への拡張が本仕様の核心。

### 1.3 依存関係ルール（A-I-P-S）

```text
L4 pricer_risk → L3 pricer_pricing → L2 pricer_models → L1 pricer_core
```

Shadow Trait は **L4 pricer_risk::enzyme** に配置（Requirement 8.1）。マーケット構造体は L2 pricer_models に存在するため、Shadow 実装は：
- **Option A**: pricer_risk で外部 impl（orphan rule 制約あり）
- **Option B**: pricer_models に Shadow trait を定義し、pricer_risk で使用
- **Option C**: pricer_risk 内にマーケット構造体のラッパーを作成

---

## 2. 要件実現可能性分析

### 2.1 技術要件と現状のギャップ

| 要件 | 現状 | ギャップ | 対応難易度 |
|------|------|----------|------------|
| **R1: Shadow Trait** | 存在しない | 新規作成 | 低 |
| **R2: スライスベースカーネル** | スカラーのみ | 新規パターン | 中 |
| **R3: AAD バインダー** | 存在しない | 新規作成 | 中 |
| **R4: ゼロコピー** | N/A | 新規設計 | 低 |
| **R5: ジェネリクス回避** | **Curve<T>** 使用中 | **重大ギャップ** | 高（要調査）|
| **R6: 勾配マッピング** | N/A | 新規設計 | 低 |
| **R7: 部分微分** | Activity 列挙型あり | 拡張必要 | 中 |
| **R8: pricer_risk 統合** | enzyme モジュール存在 | 追加統合 | 低 |

### 2.2 重大ギャップ: ジェネリクス型パラメータ

**問題**: 既存マーケット構造体は `<T: Float>` ジェネリクスを使用。
- `InterpolatedCurve<T>`, `CreditCurve<T>`, `FlatCurve<T>`

**要件 5 との矛盾**: 「ジェネリクス型パラメータを追加しない」

**解決オプション**:

| オプション | アプローチ | トレードオフ |
|-----------|-----------|--------------|
| **A: 専用マーケット構造体** | AAD 専用に `f64` 固定の構造体を新規作成 | ✅ 既存コード影響なし / ❌ 重複コード |
| **B: Shadow 用ラッパー** | 既存構造体を `f64` 特殊化でラップ | ✅ 既存構造体再利用 / ❌ 間接層追加 |
| **C: カーネル境界で f64 抽出** | ジェネリクス構造体から `&[f64]` を抽出してカーネルへ | ✅ 最小変更 / ⚠️ 要研究 |

**推奨**: **オプション C** - プロジェクト説明で示唆されたアプローチと一致。

### 2.3 Enzyme スライス対応（要研究）

**不明点**: Rust の `#[autodiff]` マクロが `&[f64]` スライス引数をサポートするか。

現在の wrappers.rs では:
```rust
#[autodiff(d_price_all, Reverse, Duplicated, Const, Duplicated, Duplicated, Duplicated, Active)]
pub fn price_european_call_adjoint(spot: f64, strike: f64, ...) -> f64
```

**必要な形式**:
```rust
#[autodiff(d_pricing_kernel, Reverse, Duplicated, Const, ...)]
pub fn pricing_kernel(rates: &[f64], times: &[f64], ...) -> f64
```

**研究必要**: Enzyme Rust バインディングのスライスサポート状況

---

## 3. 実装アプローチオプション

### Option A: 既存コンポーネント拡張

**対象ファイル**:
- `pricer_risk/src/enzyme/mod.rs` - Shadow trait 追加
- `pricer_risk/src/enzyme/greeks.rs` - GreeksEnzyme 拡張

**アプローチ**:
1. `mod.rs` に Shadow trait を定義
2. 既存 `Activity` 列挙型を活用
3. `GreeksEnzyme` を拡張してマーケット構造体対応

**トレードオフ**:
- ✅ 既存インフラ再利用
- ✅ 一貫した API
- ❌ モジュール肥大化リスク
- ❌ 単一責任原則への懸念

### Option B: 新規モジュール作成

**新規ファイル**:
- `pricer_risk/src/enzyme/shadow.rs` - Shadow trait
- `pricer_risk/src/enzyme/kernel.rs` - スライスベースカーネル
- `pricer_risk/src/enzyme/binder.rs` - AAD バインダー

**アプローチ**:
1. 明確な責任分離
2. 専用モジュールで段階的開発
3. 既存コードへの影響最小化

**トレードオフ**:
- ✅ クリーンな分離
- ✅ 独立テスト可能
- ❌ ファイル数増加
- ❌ インポートパス長期化

### Option C: ハイブリッドアプローチ（推奨）

**フェーズ 1: 基盤**
- `shadow.rs` に Shadow trait と基本実装
- 既存 `Activity` を再利用

**フェーズ 2: カーネル**
- `kernel.rs` にスライスベースカーネル
- `#[no_mangle]` は Enzyme 要件に応じて追加

**フェーズ 3: 統合**
- `binder.rs` で高レベル API 提供
- 既存 `GreeksEnzyme` との互換性確保

**トレードオフ**:
- ✅ 段階的デリバリー
- ✅ リスク分散
- ⚠️ 計画の複雑化

---

## 4. 実装複雑度とリスク

### 4.1 工数見積もり

| コンポーネント | 工数 | 根拠 |
|---------------|------|------|
| Shadow Trait 定義 | **S** (1-2日) | 単純な trait、Clone ベース |
| スライスカーネル | **M** (3-5日) | Enzyme スライス対応調査含む |
| AAD バインダー | **M** (3-5日) | 複数構造体対応 |
| 統合テスト | **M** (2-3日) | 検証ロジック |
| ドキュメント | **S** (1日) | API ドキュメント |

**合計**: **M-L** (10-16日)

### 4.2 リスク評価

| リスク | 確率 | 影響 | 緩和策 |
|--------|------|------|--------|
| Enzyme スライス未対応 | 中 | 高 | 早期技術検証、FFI フォールバック |
| ジェネリクス互換性問題 | 低 | 中 | f64 特殊化で回避 |
| パフォーマンス劣化 | 低 | 中 | ベンチマーク早期導入 |
| 既存 API 破壊 | 低 | 高 | 新規モジュール分離 |

**全体リスク**: **中**

---

## 5. 設計フェーズへの推奨事項

### 5.1 優先研究項目

1. **Enzyme #[autodiff] スライスサポート**: Rust Enzyme ドキュメント/ソースコード調査
2. **#[no_mangle] 必要性**: 要件 2.5 の FFI 要件検証
3. **Clone + zero_out パフォーマンス**: 大規模マーケットデータでのオーバーヘッド測定

### 5.2 設計決定ポイント

| 決定事項 | 選択肢 | 推奨 |
|---------|--------|------|
| Shadow Trait 配置 | pricer_risk vs pricer_models | pricer_risk（L4 封じ込め）|
| カーネル関数形式 | free fn vs method | free fn（Enzyme 互換性）|
| Activity 拡張 | 既存流用 vs 新規型 | 既存流用（Duplicated 活用）|
| マーケット構造体 | 既存再利用 vs AAD 専用 | 既存からスライス抽出 |

### 5.3 次フェーズへの引き継ぎ事項

- [ ] Enzyme Rust `#[autodiff]` のスライスサポート調査結果
- [ ] `#[no_mangle]` 要件の技術検証
- [ ] Shadow trait の nested struct サポート実装方針
- [ ] 部分微分（R7）の Activity フラグ設計

---

## 6. 要件-アセットマップ

| 要件 | 既存アセット | ステータス |
|------|-------------|-----------|
| R1 Shadow Trait | なし | **Missing** |
| R2 スライスカーネル | なし | **Missing** |
| R3 AAD バインダー | なし | **Missing** |
| R4 ゼロコピー | なし | **Missing** |
| R5 ジェネリクス回避 | Curve<T> 使用中 | **Constraint** |
| R6 勾配マッピング | なし | **Missing** |
| R7 部分微分 | Activity 列挙型 | 拡張必要 |
| R8 pricer_risk 統合 | enzyme モジュール | 追加統合 |

---

_Generated: 2026-01-26_
_Spec: shadow-object-aad_
_Language: ja_
