# Gap Analysis: new-option-pricer

## 1. 既存インフラストラクチャ調査

### 1.1 解析プライシング基盤

| コンポーネント | ファイル | 状態 |
|--------------|---------|------|
| `norm_cdf`, `norm_pdf` | `pricer_models/src/analytical/distributions.rs` | ✅ 完成（AD互換） |
| Black-Scholes公式 | `pricer_models/src/analytical/` | ❌ 未実装（モジュール構造のみ） |
| 解析ギリシャ | N/A | ❌ 未実装 |

**所見**: 正規分布関数は高精度なAbramowitz-Stegun近似で実装済み。Float型ジェネリックでAD互換。Black-Scholes公式の追加は容易。

### 1.2 ソルバー基盤

| コンポーネント | ファイル | 状態 |
|--------------|---------|------|
| `NewtonRaphsonSolver` | `pricer_core/src/math/solvers/newton_raphson.rs` | ✅ 完成（AD対応） |
| `BrentSolver` | `pricer_core/src/math/solvers/brent.rs` | ✅ 完成 |
| `SolverConfig` | `pricer_core/src/math/solvers/config.rs` | ✅ 完成 |
| `SolverError` | `pricer_core/src/types/error.rs` | ✅ 完成 |

**所見**: Newton-Raphsonソルバーは`find_root_ad`メソッドでDual64による自動微分をサポート。IV計算に直接利用可能。Brentソルバーは収束しない場合のフォールバックとして利用可能。

### 1.3 確率モデル基盤

| コンポーネント | ファイル | 状態 |
|--------------|---------|------|
| `StochasticModel` trait | `pricer_models/src/models/stochastic.rs` | ✅ 完成 |
| `SingleState<T>` | 同上 | ✅ 完成 |
| `TwoFactorState<T>` | 同上 | ✅ 完成（Heston用に設計済み） |
| `StochasticModelEnum` | `pricer_models/src/models/model_enum.rs` | ✅ 完成（拡張可能） |
| `GBMModel` | `pricer_models/src/models/gbm.rs` | ✅ 完成 |
| `HestonModel` | N/A | ❌ プレースホルダーのみ |

**所見**: `TwoFactorState`は既にHeston用に設計済み（first=価格、second=分散）。`StochasticModelEnum`にHestonバリアントを追加し、`evolve_step`を実装するだけで統合可能。

### 1.4 パス依存インフラ

| コンポーネント | ファイル | 状態 |
|--------------|---------|------|
| `PathDependentPayoff` trait | `pricer_kernel/src/path_dependent/payoff.rs` | ✅ 完成 |
| `PathObserver` | `pricer_kernel/src/path_dependent/observer.rs` | ✅ 完成 |
| `ObservationType` | `pricer_kernel/src/path_dependent/payoff.rs` | ✅ 完成 |
| Asian/Barrier/Lookback | `pricer_kernel/src/path_dependent/` | ✅ 完成 |

**所見**: 既存のパス依存インフラは単一資産用。バスケットオプションには複数資産のパス生成とペイオフ計算の拡張が必要。

### 1.5 エラー型

| エラー型 | 必要な拡張 |
|---------|-----------|
| `PricingError` | `ArbitrageBoundsViolation`, `InvalidCorrelationMatrix` 追加 |
| `SolverError` | 拡張不要 |

---

## 2. 要件実現可能性分析

### Requirement 1: Black-Scholes解析プライシング

| 技術要件 | 既存資産 | ギャップ |
|---------|---------|---------|
| BS価格計算 | `norm_cdf` | 公式実装のみ |
| AD互換性 | Float型ジェネリック | ギャップなし |
| 解析ギリシャ | `norm_pdf` | 公式実装のみ |
| 満期≤0処理 | N/A | 分岐ロジック追加 |

**複雑度**: **S (1-3日)** - 既存パターンの拡張、最小限の依存関係

### Requirement 2: インプライドボラティリティソルバー

| 技術要件 | 既存資産 | ギャップ |
|---------|---------|---------|
| Newton-Raphson | `NewtonRaphsonSolver` | ギャップなし |
| Brentフォールバック | `BrentSolver` | 統合ロジックのみ |
| AD自動微分 | `find_root_ad` | ギャップなし |
| アービトラージ境界 | N/A | バリデーション追加 |
| バッチ処理 | `rayon` | 並列化実装 |

**複雑度**: **M (3-7日)** - 複数ソルバーの統合、エラーハンドリング

### Requirement 3: Hestonモデル

| 技術要件 | 既存資産 | ギャップ |
|---------|---------|---------|
| StochasticModel実装 | トレイト定義済み | 実装のみ |
| TwoFactorState | 実装済み | ギャップなし |
| Milstein離散化 | N/A | 数値スキーム実装 |
| Full Truncation | N/A | 非負性保証実装 |
| Feller条件検証 | N/A | バリデーション追加 |
| Smooth approximation | `smooth_max`等 | 適用のみ |

**複雑度**: **M (3-7日)** - 新パターン（2因子）だが明確な設計

**Research Needed**: Milstein離散化の相関ブラウン運動への適用

### Requirement 4: バスケットオプション

| 技術要件 | 既存資産 | ギャップ |
|---------|---------|---------|
| 複数資産パス生成 | 単一資産のみ | **大幅な拡張必要** |
| 相関行列 | N/A | Cholesky分解実装 |
| コリレーテッドBM | N/A | **新規実装必要** |
| PathDependentPayoff | トレイト定義済み | 多次元拡張 |
| 正定値検証 | N/A | 線形代数実装 |

**複雑度**: **L (1-2週間)** - 複数の新規実装、アーキテクチャ変更

**Research Needed**:
- Cholesky分解の実装方法（自前 vs 外部クレート）
- 多次元パス生成のメモリ効率

### Requirement 5: テストとベンチマーク

| 技術要件 | 既存資産 | ギャップ |
|---------|---------|---------|
| 単体テスト | 既存パターン | パターン適用 |
| criterion ベンチマーク | 既存設定 | パターン適用 |
| Enzyme検証 | 既存テスト | パターン適用 |

**複雑度**: **S (1-3日)** - 既存パターンの適用

---

## 3. 実装アプローチオプション

### Option A: 既存コンポーネント拡張

**対象**: Requirement 1, 2, 3

**拡張対象ファイル**:
- `pricer_models/src/analytical/mod.rs` - BS、IV追加
- `pricer_models/src/models/model_enum.rs` - Heston追加
- `pricer_core/src/types/error.rs` - 新エラー型追加

**トレードオフ**:
- ✅ 最小限の新ファイル
- ✅ 既存パターン活用
- ❌ analytical/models モジュールの肥大化

### Option B: 新規コンポーネント作成

**対象**: Requirement 4（バスケットオプション）

**新規作成**:
- `pricer_kernel/src/multi_asset/` - 多次元パス生成
- `pricer_kernel/src/path_dependent/basket.rs` - バスケットペイオフ
- `pricer_core/src/math/linalg/` - Cholesky分解

**トレードオフ**:
- ✅ 明確な責務分離
- ✅ 単一資産コードへの影響なし
- ❌ 新モジュールの設計・統合コスト

### Option C: ハイブリッドアプローチ（推奨）

**フェーズ1**: Requirement 1, 2, 3（既存拡張）
- Black-Scholes: `pricer_models/src/analytical/black_scholes.rs`
- IV Solver: `pricer_models/src/analytical/implied_volatility.rs`
- Heston: `pricer_models/src/models/heston.rs`

**フェーズ2**: Requirement 4（新規モジュール）
- 多資産サポートを独立モジュールとして追加
- 既存単一資産コードとの互換性維持

**トレードオフ**:
- ✅ リスク分散（フェーズ分割）
- ✅ 早期価値提供（BS/IV/Hestonを先行リリース）
- ❌ 計画の複雑化

---

## 4. 実装複雑度とリスク評価

| 要件 | 工数 | リスク | 根拠 |
|-----|------|-------|-----|
| 1. Black-Scholes | S | Low | 既存パターン、明確な公式 |
| 2. IV Solver | M | Medium | 複数ソルバー統合、エッジケース処理 |
| 3. Heston | M | Medium | 新離散化スキーム、数値安定性 |
| 4. Basket | L | High | 多次元拡張、アーキテクチャ変更 |
| 5. Tests | S | Low | 既存パターン適用 |

**総合評価**: M-L（3週間〜1ヶ月）

---

## 5. 設計フェーズへの推奨事項

### 推奨アプローチ
**Option C（ハイブリッド）** を推奨。フェーズ1でBS/IV/Hestonを実装し、フェーズ2でバスケットオプションを追加。

### Research Items（設計フェーズで調査）
1. Milstein離散化の相関ブラウン運動への拡張方法
2. Cholesky分解の実装（`nalgebra`クレート利用 vs 自前実装）
3. 多次元パス生成のメモリ効率最適化

### 主要決定事項
1. **Hestonの離散化スキーム**: Milstein vs Euler-Maruyama vs QE（Quadratic Exponential）
2. **Cholesky分解の依存**: 外部クレート依存 vs 最小限の自前実装
3. **多資産パスのメモリレイアウト**: AoS vs SoA

### 既存アーキテクチャとの整合性
- **静的ディスパッチ維持**: `StochasticModelEnum`パターンを踏襲
- **AD互換性維持**: すべての新実装でFloat型ジェネリックを使用
- **Smooth approximation**: 不連続点はすべてsmooth関数で置換
