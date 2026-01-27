# Gap Analysis: remove-dual-ad

## Executive Summary

Dual Number（num-dual）方式の自動微分を削除し、Enzyme方式に統一するリファクタリングのギャップ分析。

### 主要発見事項

- **影響範囲**: 限定的（主にpricer_core、一部pricer_models）
- **実コード使用**: 2箇所の実装使用（newton_raphson.rs、vega.rs）
- **ドキュメント参照**: 約70ファイルでDual64/DualNumberへの言及（主にdocコメント）
- **リスク**: 低〜中（Enzyme fallbackへの依存度確認が必要）

---

## 1. Current State Investigation

### 1.1 Dual Number関連ファイル

| ファイル | 役割 | LOC | 影響度 |
|---------|------|-----|--------|
| `pricer_core/src/types/dual.rs` | DualNumber型エイリアス定義 | 58 | 高（削除対象） |
| `pricer_core/src/types/mod.rs` | dualモジュール条件付きエクスポート | 51 | 中（修正必要） |
| `pricer_core/tests/dual_number.rs` | DualNumber単体テスト | 251 | 高（削除対象） |
| `pricer_core/tests/module_exports.rs` | num-dual-modeフィーチャーテスト | 60 | 中（修正必要） |

### 1.2 Feature Flag構造

```
Cargo.toml (workspace)
└── num-dual = "0.9"

pricer_core/Cargo.toml
├── num-dual = { workspace = true, optional = true }
└── features:
    ├── default = ["num-dual-mode", ...]
    └── num-dual-mode = ["dep:num-dual"]

pricer_models/Cargo.toml
└── num-dual-mode = ["pricer_core/num-dual-mode"]
```

### 1.3 実コード使用箇所（Critical）

#### 1.3.1 Newton-Raphson Solver（AD対応版）

**ファイル**: `pricer_core/src/math/solvers/newton_raphson.rs:157-200`

```rust
#[cfg(feature = "num-dual-mode")]
pub fn solve_ad<F>(&self, f: F, initial_guess: f64) -> Result<f64, SolverError>
where
    F: Fn(num_dual::Dual64) -> num_dual::Dual64,
{
    use num_dual::Dual64;
    // ...
}
```

**対応策**: Enzyme forward modeまたはfinite difference fallbackに置換

#### 1.3.2 VolCube Vega計算（Forward AD）

**ファイル**: `pricer_models/src/market/volcube/vega.rs:523-547`

```rust
#[cfg(feature = "num-dual-mode")]
pub fn compute_vega_forward_ad<F>(&self, vol: f64, pricing_fn: F) -> f64
where
    F: Fn(num_dual::Dual64) -> num_dual::Dual64,
{
    use num_dual::DualNum;
    let vol_dual = num_dual::Dual64::from(vol).derivative();
    let result = pricing_fn(vol_dual);
    result.eps
}
```

**対応策**: Enzyme forward modeに置換、またはfinite difference fallbackを使用

### 1.4 GreeksMode列挙型

**ファイル**: `pricer_risk/src/greeks/config.rs:17-40`

```rust
pub enum GreeksMode {
    #[default]
    BumpRevalue,
    NumDual,  // ← 削除または非推奨化
    #[cfg(feature = "enzyme-ad")]
    EnzymeAAD,
}
```

### 1.5 ドキュメント参照（約70ファイル）

多くのファイルで型パラメータ説明に「e.g., `f64`, `Dual64`」の記述あり：
- `pricer_core/src/math/interpolators/*.rs`
- `pricer_models/src/market/curves/*.rs`
- `pricer_models/src/market/surfaces/*.rs`
- `pricer_models/src/analytical/*.rs`
- etc.

→ **ドキュメントのみの変更**（実装への影響なし）

### 1.6 CI/CD設定

**ファイル**: `.github/workflows/ci.yml`

- Line 69: `cargo test -p pricer_core --features num-dual-mode`
- Line 155: `"⚠ CI ran with num-dual fallback (Enzyme not available)"`

---

## 2. Requirements Feasibility Analysis

### 2.1 技術要件マッピング

| 要件 | 現状資産 | ギャップ | 対応策 |
|------|---------|---------|--------|
| R1: dual.rs削除 | 存在 | なし | ファイル削除 |
| R2: num-dual依存削除 | 3箇所 | なし | Cargo.toml編集 |
| R3: 依存コード更新 | 2関数 | 代替実装必要 | Enzyme/FD |
| R4: テスト更新 | 251 LOC | 削除または変換 | 要判断 |
| R5: ドキュメント更新 | 70+ファイル | 一括置換可能 | sed/grep |
| R6: 後方互換性 | Public API | solve_ad削除 | Breaking change |

### 2.2 制約事項

1. **Enzyme依存**: Enzyme未インストール環境ではGreeks計算にfinite differenceのみ
2. **solve_ad関数**: Public APIのため削除はbreaking change
3. **compute_vega_forward_ad**: pricer_models内部APIだが使用箇所確認必要

### 2.3 Research Needed

- [ ] `solve_ad`のユーザー使用状況（demo, service_python等）
- [ ] Enzyme forward modeの`solve_ad`相当実装の可否
- [ ] `GreeksMode::NumDual`選択時の既存動作（エラー or fallback）

---

## 3. Implementation Approach Options

### Option A: 完全削除（Recommended）

**アプローチ**: Dual Number関連コードを完全削除し、Enzyme + BumpRevalueに統一

**変更対象**:
1. 削除: `pricer_core/src/types/dual.rs`
2. 削除: `pricer_core/tests/dual_number.rs`
3. 修正: `pricer_core/src/types/mod.rs`（dualモジュール参照削除）
4. 修正: `pricer_core/Cargo.toml`（num-dual依存、num-dual-mode feature削除）
5. 修正: `pricer_models/Cargo.toml`（num-dual-mode feature削除）
6. 修正: `pricer_core/src/math/solvers/newton_raphson.rs`（solve_ad削除）
7. 修正: `pricer_models/src/market/volcube/vega.rs`（compute_vega_forward_ad削除）
8. 修正: `pricer_risk/src/greeks/config.rs`（GreeksMode::NumDual削除）
9. 修正: `.github/workflows/ci.yml`（num-dual関連テスト削除）
10. 更新: `Cargo.toml`（workspace num-dual依存削除）
11. 更新: ドキュメント（約70ファイルのdocコメント）
12. 更新: steering文書（tech.md, structure.md, product.md）

**Trade-offs**:
- ✅ コードベース大幅簡素化
- ✅ 依存関係削減
- ✅ 保守コスト削減
- ❌ Breaking change（solve_ad削除）
- ❌ Enzyme未環境でAD機能制限

### Option B: 非推奨化（Conservative）

**アプローチ**: `#[deprecated]`属性で段階的移行

**変更対象**:
1. 修正: `pricer_core/src/types/dual.rs`（`#[deprecated]`追加）
2. 修正: `solve_ad`、`compute_vega_forward_ad`に`#[deprecated]`
3. 修正: `GreeksMode::NumDual`に`#[deprecated]`
4. 追加: 移行ガイドドキュメント

**Trade-offs**:
- ✅ 後方互換性維持
- ✅ ユーザー移行猶予
- ❌ 技術的負債継続
- ❌ 2つのADバックエンド保守

### Option C: Hybrid（Enzyme Wrapper）

**アプローチ**: `solve_ad`をEnzyme wrapperとして再実装

**変更対象**:
1. 移動: `solve_ad`を`pricer_risk/src/enzyme/`へ
2. 実装: Enzyme forward modeによる自動微分Newton-Raphson
3. 削除: `num-dual`依存（`solve_ad`以外）

**Trade-offs**:
- ✅ API互換性維持
- ✅ num-dual削除可能
- ❌ Enzyme nightly必須化
- ❌ 実装複雑度増加

---

## 4. Implementation Complexity & Risk

### Effort: **S-M（1-5日）**

- 主要変更: 10-12ファイル
- ドキュメント更新: 70+ファイル（一括置換可能）
- テスト影響: 251 LOC削除 + 周辺テスト確認

**内訳**:
| タスク | 工数 |
|--------|------|
| 核心コード削除 | 0.5日 |
| Cargo.toml/feature整理 | 0.5日 |
| CI/CD更新 | 0.5日 |
| ドキュメント一括更新 | 1日 |
| テスト・検証 | 1-2日 |

### Risk: **Medium**

| リスク要因 | 影響 | 緩和策 |
|-----------|------|--------|
| solve_ad Breaking change | 外部ユーザー影響 | CHANGELOGで明示、代替案提示 |
| Enzyme未環境 | AD機能制限 | BumpRevalue fallback維持 |
| 見落としimport | コンパイルエラー | CI全ターゲットテスト |
| ドキュメント整合性 | 混乱 | 一括sed置換で統一 |

---

## 5. Recommendations for Design Phase

### 推奨アプローチ: **Option A（完全削除）**

**理由**:
1. Enzyme方式が成熟し、num-dualは事実上不要
2. コードベース簡素化による長期保守コスト削減
3. Breaking changeは限定的（solve_ad関数のみ）

### 設計フェーズで決定すべき事項

1. **solve_ad代替策**: Enzyme forward mode実装 or ドキュメント記載のみ
2. **GreeksMode::NumDual**: 即時削除 or `#[deprecated]`経由
3. **ドキュメント更新戦略**: `Dual64`言及の置換先テキスト
4. **移行ガイド**: CHANGELOGエントリの詳細度

### Research Items（設計フェーズへ繰越）

- [ ] `solve_ad`の実使用状況調査（demo, service_python, 外部）
- [ ] Enzyme forward mode Newton-Raphsonの実装可能性
- [ ] Enzyme未環境でのVega計算代替案（BumpRevalue精度）

---

_Generated: 2026-01-27_
