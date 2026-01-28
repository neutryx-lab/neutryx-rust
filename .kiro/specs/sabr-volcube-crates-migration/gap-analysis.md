# Gap Analysis: SABR VolCube Crates Migration

## 概要

本ドキュメントは、`sabr-volcube-crates-migration`仕様の要件と既存コードベースとの差分を分析する。

---

## 要件マッピング

### Requirement 1: VolQuoteデータ構造の拡張

| 受入基準 | 既存アセット | ギャップ | タグ |
|----------|------------|---------|------|
| 1.1 VolQuote shall have expiry field | [mod.rs:137-144](crates/pricer_models/src/builder/vol/mod.rs#L137-L144): `strike, volatility, forward`のみ | `expiry: T`フィールドが不足 | GAP |
| 1.2 new() shall accept 4 params | [mod.rs:148-150](crates/pricer_models/src/builder/vol/mod.rs#L148-L150): 3パラメータ | 4パラメータ版が必要 | GAP |
| 1.3 new_without_expiry() | 存在しない | 後方互換コンストラクタが必要 | GAP |
| 1.4 既存テスト更新 | [mod.rs:335-339](crates/pricer_models/src/builder/vol/mod.rs#L335-L339) | テストのシグネチャ更新が必要 | GAP |

**影響範囲**: `pricer_models::builder::vol::VolQuote`

---

### Requirement 2: SliceCalibrationConfigの拡張

| 受入基準 | 既存アセット | ギャップ | タグ |
|----------|------------|---------|------|
| 2.1 追加フィールド | [mod.rs:175-184](crates/pricer_models/src/builder/vol/mod.rs#L175-L184): `fixed_beta, max_iterations, tolerance, initial_alpha` | `initial_rho, initial_nu, lm_lambda, lm_lambda_factor, bounds`が不足 | GAP |
| 2.2 SabrBounds構造体 | 存在しない | 新規作成が必要 | GAP |
| 2.3 SabrBounds::default() | N/A | 新規実装が必要 | GAP |
| 2.4 SliceCalibrationConfig::default() | [mod.rs:186-195](crates/pricer_models/src/builder/vol/mod.rs#L186-L195) | 追加フィールドのデフォルト値が必要 | GAP |
| 2.5 rates() preset | [mod.rs:199-204](crates/pricer_models/src/builder/vol/mod.rs#L199-L204) | 拡張フィールドのrates用設定が必要 | PARTIAL |
| 2.6 fx() preset | [mod.rs:207-212](crates/pricer_models/src/builder/vol/mod.rs#L207-L212) | 拡張フィールドのFX用設定が必要 | PARTIAL |

**影響範囲**: `pricer_models::builder::vol::SliceCalibrationConfig`

---

### Requirement 3: SabrCalibrationProblemの実装

| 受入基準 | 既存アセット | ギャップ | タグ |
|----------|------------|---------|------|
| 3.1 LMProblem trait実装 | [levenberg_marquardt.rs](crates/pricer_core/src/math/solvers/levenberg_marquardt.rs): **クロージャベースAPI** | **要件修正必要**: LMソルバーはトレイトベースではなくクロージャ`Fn(&[f64]) -> Vec<f64>`を使用 | DESIGN_CHANGE |
| 3.2 quotes, beta, boundsコンストラクタ | N/A | クロージャでキャプチャ可能 | GAP |
| 3.3 num_params() = 3 | N/A | クロージャ内で暗黙的 | N/A |
| 3.4 num_residuals() | N/A | クロージャ内で暗黙的 | N/A |
| 3.5 residuals計算 | N/A | クロージャ本体で実装 | GAP |
| 3.6 jacobian数値計算 | [levenberg_marquardt.rs:354-378](crates/pricer_core/src/math/solvers/levenberg_marquardt.rs#L354-L378): `compute_jacobian`関数 | **ソルバー内部で自動計算済み** | OK |
| 3.7 sabr_implied_vol使用 | [sabr.rs](crates/pricer_core/src/math/formulas/sabr.rs): 完全なHagan公式 | 使用可能 | OK |

**重要な設計変更**:
要件では`LMProblem<T>`トレイトを想定していたが、pricer_coreのLMソルバーはクロージャベースAPI:
```rust
pub fn solve<F>(&self, residuals: F, initial_params: Vec<f64>) -> Result<LMResult, SolverError>
where F: Fn(&[f64]) -> Vec<f64>
```
これは実装上の問題ではなく、むしろ柔軟性が高い。`SabrCalibrationProblem`構造体は不要で、代わりにクロージャを直接構築する。

---

### Requirement 4: SabrSliceCalibratorの完全実装

| 受入基準 | 既存アセット | ギャップ | タグ |
|----------|------------|---------|------|
| 4.1 LMソルバー使用 | [mod.rs:265-267](crates/pricer_models/src/builder/vol/mod.rs#L265-L267): **TODOプレースホルダー** | LM統合が必要 | GAP |
| 4.2 空quotes → InsufficientData | [mod.rs:245-250](crates/pricer_models/src/builder/vol/mod.rs#L245-L250) | **実装済み** | OK |
| 4.3 非収束 → NonConvergence | N/A | `ConvergenceFailure`で対応可能 | GAP |
| 4.4 成功時 → SabrParams返却 | [mod.rs:269-270](crates/pricer_models/src/builder/vol/mod.rs#L269-L270) | validate済みで返却済み（但し固定値） | PARTIAL |
| 4.5 ATMからalpha推定 | [mod.rs:253-263](crates/pricer_models/src/builder/vol/mod.rs#L253-L263) | **実装済み** | OK |
| 4.6 bounds適用 | N/A | クロージャ内でclamp処理が必要 | GAP |
| 4.7 validate()呼び出し | [mod.rs:269](crates/pricer_models/src/builder/vol/mod.rs#L269) | **実装済み** | OK |

**核心的ギャップ**: `calibrate_slice()`本体でLMソルバーを呼び出し、最適化を実行するロジックが必要。

---

### Requirement 5: VolCubeBuilder APIの更新

| 受入基準 | 既存アセット | ギャップ | タグ |
|----------|------------|---------|------|
| 5.1 add_quote() 5パラメータ | [cube.rs](crates/pricer_models/src/builder/vol/cube.rs)の調査が必要 | expiryパラメータ追加が必要 | GAP |
| 5.2 add_slice() | 同上 | expiry, tenorパラメータ追加が必要 | GAP |
| 5.3 calibrate()でSabrSliceCalibrator使用 | 同上 | 実装状態の確認が必要 | TBD |
| 5.4 VolCubeResult返却 | 同上 | 実装状態の確認が必要 | TBD |

---

### Requirement 6: CalibrationErrorの拡張

| 受入基準 | 既存アセット | ギャップ | タグ |
|----------|------------|---------|------|
| 6.1 OptimisationFailed | [error.rs:60-64](crates/pricer_models/src/builder/error.rs#L60-L64): `NumericalInstability { message }` | **既存で代替可能** | OK |
| 6.2 NonConvergence | [error.rs:24-29](crates/pricer_models/src/builder/error.rs#L24-L29): `ConvergenceFailure { iterations, residual }` | **既存で完全対応** | OK |
| 6.3 LMエラーラップ | `NumericalInstability`で対応 | OK | OK |
| 6.4 最大イテレーション超過 | `ConvergenceFailure`で対応 | OK | OK |

**結論**: CalibrationErrorは既に十分なエラーバリアントを持っている。新規追加不要。

---

### Requirement 7: demo/guiリファクタリング

| 受入基準 | 既存アセット | ギャップ | タグ |
|----------|------------|---------|------|
| 7.1 スタンドアロン関数削除 | [volcube.rs](demo/gui/src/web/handlers/volcube.rs): `calibrate_sabr_simple`, `optimize_sabr`, `sabr_implied_vol`, `black_call_price` | 削除が必要 | GAP |
| 7.2 VolCubeBuilder使用 | 現在はスタンドアロン実装 | 移行が必要 | GAP |
| 7.3 SliceCalibrationConfig構築 | N/A | 新規実装が必要 | GAP |
| 7.4 HTTPレスポンス変換 | 現在は独自エラー処理 | CalibrationError対応が必要 | GAP |
| 7.5 pricer_models依存追加 | 要確認 | Cargo.toml更新が必要 | TBD |

---

### Requirement 8: 単体テストと検証

| 受入基準 | 既存アセット | ギャップ | タグ |
|----------|------------|---------|------|
| 8.1-8.7 各種テスト | [mod.rs:300-349](crates/pricer_models/src/builder/vol/mod.rs#L300-L349): 基本テストのみ | カリブレーション精度テストが不足 | GAP |

---

### Requirement 9: 性能と収束性

| 受入基準 | 既存アセット | ギャップ | タグ |
|----------|------------|---------|------|
| 9.1 100イテレーション以内 | [levenberg_marquardt.rs:86](crates/pricer_core/src/math/solvers/levenberg_marquardt.rs#L86): `max_iterations: 100` | **デフォルトで対応** | OK |
| 9.2 tolerance=1e-8 | [mod.rs:191](crates/pricer_models/src/builder/vol/mod.rs#L191): `tolerance: 1e-8` | **デフォルトで対応** | OK |
| 9.3 50イテレーション以内収束 | 実装後に検証 | テストで確認 | TBD |
| 9.4 再現可能性 | LMソルバーは決定論的 | OK | OK |

---

## 実装オプション

### Option A: 既存LMソルバーを直接使用（推奨）

**アプローチ**:
- `SabrCalibrationProblem`トレイトを作成せず、クロージャで残差関数を構築
- `SliceCalibrationConfig`を拡張し、`LMConfig`への変換メソッドを追加
- `SabrSliceCalibrator::calibrate_slice()`内でクロージャを構築してLMソルバーに渡す

**利点**:
- 既存のpricer_core APIをそのまま活用
- 最小限のコード変更
- LMソルバーの内部ヤコビアン計算を活用

**実装イメージ**:
```rust
fn calibrate_slice(&self, quotes: &[VolQuote<T>], config: &SliceCalibrationConfig<T>)
    -> Result<SabrParams<T>, CalibrationError>
{
    let beta = config.fixed_beta.unwrap_or(from_f64(0.5));
    let forward = quotes[0].forward;
    let expiry = quotes[0].expiry;

    let residuals = |params: &[f64]| -> Vec<f64> {
        let alpha = params[0].clamp(bounds.alpha_min, bounds.alpha_max);
        let rho = params[1].clamp(bounds.rho_min, bounds.rho_max);
        let nu = params[2].clamp(bounds.nu_min, bounds.nu_max);

        quotes.iter().map(|q| {
            let model_vol = sabr_implied_vol(forward, alpha, beta, rho, nu, expiry, q.strike);
            q.volatility - model_vol
        }).collect()
    };

    let lm_config = LMConfig { tolerance, max_iterations, initial_lambda, .. };
    let solver = LevenbergMarquardtSolver::new(lm_config);
    let result = solver.solve(residuals, initial_params)?;

    if !result.converged {
        return Err(CalibrationError::convergence_failure(result.iterations, result.residual_ss));
    }

    Ok(SabrParams::new(result.params[0], beta, result.params[1], result.params[2]))
}
```

---

### Option B: LMProblemトレイトを新規作成

**アプローチ**:
- `pricer_core`に新しい`LMProblem`トレイトを追加
- `LevenbergMarquardtSolver`に`solve_problem<P: LMProblem>()`メソッドを追加
- `SabrCalibrationProblem`を`LMProblem`トレイトで実装

**欠点**:
- pricer_coreへの変更が必要
- 既存のクロージャベースAPIとの整合性
- 追加の抽象化レイヤー

**非推奨**: 既存APIで十分対応可能

---

### Option C: demo/gui内の実装を移植

**アプローチ**:
- demo/guiの`optimize_sabr()`をそのまま`pricer_models`に移植
- pricer_coreのLMソルバーは使用しない

**欠点**:
- コード重複の解消にならない
- pricer_coreのテスト済みソルバーを活用しない
- メンテナンス負荷

**非推奨**: アーキテクチャ原則に反する

---

## 推奨アプローチ: Option A

### 変更サマリー

| ファイル | 変更内容 | 工数 |
|----------|---------|------|
| `pricer_models/src/builder/vol/mod.rs` | VolQuote拡張、SliceCalibrationConfig拡張、SabrBounds追加、SabrSliceCalibrator実装 | M |
| `pricer_models/src/builder/vol/cube.rs` | VolCubeBuilder API更新（add_quote, add_slice） | S |
| `demo/gui/src/web/handlers/volcube.rs` | スタンドアロン関数削除、VolCubeBuilder使用に移行 | M |
| `demo/gui/Cargo.toml` | pricer_models依存追加（必要な場合） | S |
| テスト追加 | カリブレーション精度テスト、収束テスト | M |

### 工数見積

| サイズ | 説明 |
|--------|------|
| S | 小規模変更（数行〜数十行） |
| M | 中規模変更（50-200行） |
| L | 大規模変更（200行以上） |

**総工数**: M（中規模）

---

## リスク評価

| リスク | 影響度 | 発生確率 | 緩和策 |
|--------|--------|---------|--------|
| LMソルバーとSABR公式の連携問題 | 中 | 低 | 単体テストで段階的に検証 |
| 後方互換性の破壊 | 中 | 中 | `new_without_expiry()`コンストラクタ提供 |
| カリブレーション精度不足 | 中 | 低 | demo/guiの既存実装と比較検証 |
| demo/guiの依存関係循環 | 高 | 低 | A-I-P-Sレイヤー原則に従う |

---

## 要件修正提案

### Requirement 3の修正

**現行要件**:
> The `SabrCalibrationProblem<'a, T>` shall `pricer_core::math::solvers::LMProblem<T>`トレイトを実装する

**修正提案**:
> The `SabrSliceCalibrator` shall `pricer_core::math::solvers::LevenbergMarquardtSolver`のクロージャベースAPIを使用して最適化を実行する

**理由**: pricer_coreのLMソルバーはトレイトベースではなくクロージャベースAPI `Fn(&[f64]) -> Vec<f64>` を採用している。これは要件の意図（LMソルバーとの統合）を満たしつつ、既存APIと整合する。

---

## 次のステップ

1. **要件承認**: Requirement 3の修正提案を承認
2. **設計フェーズ**: 詳細設計ドキュメントを生成
3. **タスク生成**: 実装タスクを分解
4. **実装**: TDD方式で実装
5. **検証**: demo/guiの既存実装と結果を比較

---

_Generated: 2026-01-28_
_Specification: sabr-volcube-crates-migration_
_Phase: Gap Analysis_
