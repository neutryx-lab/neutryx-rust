# Gap Analysis: models-module-refactor

## 概要

`pricer_models` クレート内の `models/` および `analytical/` モジュールの構造的見直しに関するギャップ分析。

**分析日**: 2026-01-27
**更新**: ゼロベース構成提案を反映

---

## 1. 現状調査

### 1.1 現在のディレクトリ構成

```
crates/pricer_models/src/
├── models/                 # 確率過程モデル
│   ├── mod.rs              # モジュール定義、re-exports
│   ├── stochastic.rs       # StochasticModel trait、状態型
│   ├── model_enum.rs       # StochasticModelEnum (静的ディスパッチ)
│   ├── validation.rs       # パラメータ検証
│   ├── error.rs            # ModelError
│   ├── gbm.rs              # GBM (feature = "equity")
│   ├── heston.rs           # Heston (feature = "equity")
│   ├── sabr.rs             # SABR ★ 複合 (feature = "equity")
│   ├── hull_white.rs       # Hull-White (feature = "rates")
│   ├── cir.rs              # CIR (feature = "rates")
│   └── correlated.rs       # 相関モデル (feature = "exotic")
│
├── analytical/             # 閉形式解
│   ├── mod.rs              # モジュール定義
│   ├── black_scholes.rs    # Black-Scholes
│   ├── bachelier.rs        # Bachelier (Normal model)
│   ├── garman_kohlhagen.rs # Garman-Kohlhagen (FX)
│   ├── distributions.rs    # ★ pricer_coreからのre-export (不要)
│   └── error.rs            # AnalyticalError
│
└── market/
    ├── calibration/
    │   └── sabr.rs         # SABR キャリブレーション
    └── volcube/
        ├── cube.rs         # VolCube (SABRベース)
        ├── sabr_surface.rs # SabrParameterSurface
        └── types.rs        # SabrParams (別定義あり)
```

### 1.2 主要な発見事項

| 項目 | 現状 | 備考 |
|------|------|------|
| `analytical/distributions.rs` | **既に re-export のみ** | 削除可能 |
| `models/sabr.rs` | **複合モジュール** | StochasticModel実装 + Hagan公式（約2600行） |
| SABR SDE (`evolve_step`) | **未使用** | 削除対象 |
| `market/volcube/types.rs` | 別の `SabrParams` 定義 | 統合検討 |
| `pricer_core` 分布関数 | 完全実装済み | `norm_cdf`, `norm_pdf`, `norm_inv_cdf` |

---

## 2. 承認済み構成提案

### 2.1 目標構成

```
crates/pricer_models/src/
│
├── stochastic/              # 確率過程 (MC用)
│   ├── mod.rs               # StochasticModel trait, 状態型
│   ├── gbm.rs               # Geometric Brownian Motion
│   ├── heston.rs            # Heston確率ボラティリティ
│   ├── hull_white.rs        # Hull-White短期金利
│   ├── cir.rs               # Cox-Ingersoll-Ross
│   ├── correlated.rs        # 相関モデル
│   └── model_enum.rs        # StochasticModelEnum
│
├── formulas/                # 閉形式解・解析公式 (フラット)
│   ├── mod.rs
│   ├── black_scholes.rs     # BS価格公式
│   ├── bachelier.rs         # Normal価格公式
│   ├── garman_kohlhagen.rs  # FX価格公式
│   └── sabr_implied_vol.rs  # SABR Hagan近似
│
├── market/                  # マーケットデータ (現状維持)
│   └── ...
│
└── compiler/                # IRコンパイラ (現状維持)
```

### 2.2 変更一覧

| 操作 | 対象 | 理由 |
|------|------|------|
| **名前変更** | `models/` → `stochastic/` | 確率過程という明確な意味 |
| **名前変更** | `analytical/` → `formulas/` | 公式という明確な意味 |
| **削除** | `models/sabr.rs` | SABR SDEは未使用 |
| **削除** | `analytical/distributions.rs` | `pricer_core` で十分 |
| **新規作成** | `formulas/sabr_implied_vol.rs` | Hagan公式のみを抽出 |
| **移動** | 価格公式群 | `analytical/` → `formulas/` |

---

## 3. 要件実現性分析

| 要件 | 技術的ニーズ | ギャップ | アクション |
|------|-------------|----------|----------|
| Req 1 (責務明確化) | stochastic/formulas 分離 | ディレクトリ名変更 | **実装可能** |
| Req 2 (重複排除) | distributions.rs 削除 | 削除のみ | **実装可能** |
| Req 3 (SABR分離) | Hagan公式のみ抽出 | 新規ファイル作成 | **実装可能** |
| Req 4 (L1/L2境界) | pricer_core 依存維持 | 変更なし | **OK** |
| Req 5 (後方互換) | deprecated re-export | 設計必要 | 詳細設計で対応 |
| Req 6 (テスト) | 全テスト通過 | 実装後検証 | 実装フェーズ |

---

## 4. 複雑性とリスク評価

### 工数見積り

| 項目 | 工数 |
|------|------|
| ディレクトリ名変更 | S (1日) |
| ファイル移動・削除 | S (1日) |
| sabr_implied_vol.rs 作成 | M (2日) |
| 後方互換 re-export | S (1日) |
| テスト修正・検証 | M (2日) |
| **合計** | **M (5-7日)** |

### リスク評価

| リスク | レベル | 軽減策 |
|--------|--------|--------|
| 外部依存の破壊 | Medium | deprecated re-export で段階的移行 |
| VolCube との整合性 | Low | SabrParams を formulas から参照 |
| feature flag の影響 | Low | 既存のフラグ構造を維持 |

---

## 5. 設計フェーズへの引き継ぎ事項

### 決定済み事項

1. **ディレクトリ構造**: `stochastic/` + `formulas/` (フラット)
2. **SABR SDE**: 削除（未使用のため）
3. **distributions.rs**: 削除（pricer_core を直接使用）

### 設計フェーズでの決定事項

1. **sabr_implied_vol.rs の API 設計**
   - 既存の `SABRParams`, `SABRModel::implied_vol()` からの抽出方法
   - `market/volcube/types.rs::SabrParams` との統合可否

2. **後方互換 re-export の詳細**
   - `#[deprecated]` メッセージの内容
   - 移行期間の設定

3. **ファイル移動の順序**
   - 依存関係を考慮した移動順序
   - CI/CD での検証タイミング
