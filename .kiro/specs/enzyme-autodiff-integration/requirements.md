# Requirements Document

## Introduction

本仕様は、Enzyme LLVM-level ADの `#[autodiff]` マクロを pricer_pricing クレートに本格統合し、高性能Greeks計算を実現することを目的とする。

現在の実装は有限差分近似によるプレースホルダーであり、本仕様によりRustのnightly機能である `#[autodiff]` マクロを活用した真のLLVMレベル自動微分を実現する。これにより、C++ライブラリと同等のパフォーマンスでDelta、Gamma、Vega等のGreeks計算が可能となる。

### 背景

- **現状**: `pricer_pricing::enzyme` モジュールは有限差分近似を使用（Phase 3.0 placeholder）
- **目標**: Enzyme `#[autodiff_reverse]` / `#[autodiff_forward]` マクロによる本格AD統合
- **対象レイヤー**: L3 (pricer_pricing) - nightly Rust + LLVM 18 環境

### 制約

- Enzyme対応は pricer_pricing クレートのみ（他クレートはstable Rust維持）
- nightly-2025-01-15 ツールチェーン必須
- LLVM 18 + Enzyme プラグイン環境必須
- 静的ディスパッチ（enum）パターンとの互換性維持

---

## Requirements

### Requirement 1: Nightly Feature 有効化

**Objective:** As a 開発者, I want `#![feature(autodiff)]` を有効化する仕組み, so that Enzyme `#[autodiff]` マクロが使用可能になる

1. When `enzyme-ad` feature が有効化された時, the pricer_pricing shall `#![feature(autodiff)]` を有効化する
2. When `enzyme-ad` feature が無効の時, the pricer_pricing shall stable Rust でコンパイル可能な状態を維持する
3. The pricer_pricing shall `rust-toolchain.toml` で nightly-2025-01-15 を指定する
4. If LLVM 18 が未インストールの場合, the build system shall 明確なエラーメッセージを表示する

---

### Requirement 2: Forward Mode AD 統合

**Objective:** As a クオンツ開発者, I want Forward mode AD による単一パラメータ微分, so that 個別のGreeks（Delta等）を効率的に計算できる

1. When `#[autodiff_forward]` マクロが適用された時, the enzyme module shall tangent値を入力から出力へ伝播する
2. The enzyme module shall `autodiff_forward` を使用したラッパー関数を提供する
3. When spot価格に対するtangent seed が設定された時, the pricer shall Delta値をtangent出力として返す
4. The enzyme module shall Forward mode 用の型安全なAPI (`ForwardAD<T>`) を提供する
5. While Forward mode が有効な時, the pricer shall 単一変数微分を O(1) 追加コストで計算する

---

### Requirement 3: Reverse Mode AD 統合

**Objective:** As a クオンツ開発者, I want Reverse mode AD による多変数微分, so that 全Greeksを一回のパスで効率的に計算できる

1. When `#[autodiff_reverse]` マクロが適用された時, the enzyme module shall adjoint値を出力から入力へ逆伝播する
2. The enzyme module shall `autodiff_reverse` を使用したラッパー関数を提供する
3. When 出力のadjoint seed が 1.0 に設定された時, the pricer shall 全入力パラメータに対する勾配を返す
4. The enzyme module shall Reverse mode 用の型安全なAPI (`ReverseAD<T>`) を提供する
5. While Reverse mode が有効な時, the pricer shall 全パラメータの勾配を O(出力数) 追加コストで計算する

---

### Requirement 4: Monte Carlo Pricer 統合

**Objective:** As a デリバティブトレーダー, I want MC Pricerでの自動Greeks計算, so that バニラ・エキゾチックオプションのリスク管理ができる

1. When `MonteCarloPricer::price_with_greeks()` が呼び出された時, the pricer shall Enzymeを使用してGreeksを計算する
2. The MonteCarloPricer shall Delta, Gamma, Vega, Theta, Rho の計算をサポートする
3. When path-dependent option を価格計算する時, the pricer shall チェックポインティングと組み合わせてADを適用する
4. While 並列シミュレーション中, the pricer shall スレッドセーフにGreeksを集約する
5. The MonteCarloPricer shall `GreeksResult<f64>` 構造体でGreeksを返す
6. Where `enzyme-ad` feature が無効の場合, the MonteCarloPricer shall 有限差分によるフォールバック計算を行う

---

### Requirement 5: 検証・正確性

**Objective:** As a クオンツ開発者, I want AD結果の検証機構, so that 計算の正確性を保証できる

1. The enzyme module shall Enzyme AD と num-dual の比較検証テストを提供する
2. When 検証テスト実行時, the system shall 相対誤差 1e-6 以内を確認する
3. The enzyme module shall 有限差分法との比較検証も提供する
4. The pricer_pricing shall Black-Scholes analytical Greeks との比較テストを含む
5. If AD結果と検証値の差異が閾値を超えた場合, the test system shall 警告を出力する

---

### Requirement 6: Enzyme対応関数設計

**Objective:** As a 開発者, I want Enzyme互換の関数設計ガイドライン, so that ADが正しく動作する関数を実装できる

1. The enzyme module shall Enzyme対応の `smooth_payoff` 関数群を提供する
2. The enzyme module shall 不連続性を含まない smooth approximation パターンを適用する
3. When 条件分岐が必要な場合, the system shall `smooth_indicator` または `smooth_max` を使用する
4. The enzyme module shall 固定サイズ `for` ループのみを使用する（`while` 禁止）
5. The pricer_pricing shall Enzyme非互換パターンのコンパイル時警告を提供する

---

### Requirement 7: パフォーマンス要件

**Objective:** As a トレーディングデスク, I want 高速なGreeks計算, so that リアルタイムリスク管理が可能になる

1. The enzyme-based Greeks calculation shall 有限差分法より 5倍以上高速であること
2. When 10,000パスのMCシミュレーション時, the pricer shall Greeks込みで 100ms 以内に完了すること
3. The enzyme AD shall C++ Enzyme実装と同等のパフォーマンスを達成すること
4. The pricer_pricing shall `criterion` ベンチマークでパフォーマンス回帰を検出する
5. While 大規模ポートフォリオ計算中, the system shall 線形スケーリングを維持する

---

### Requirement 8: ビルド・CI 統合

**Objective:** As a 開発者, I want Enzyme ビルドのCI統合, so that 継続的にビルドとテストが検証される

1. The CI system shall nightly + Enzyme ビルドを別ジョブで実行する
2. When Enzyme ビルドが失敗した時, the CI shall stable クレートのビルドには影響しないこと
3. The Docker environment shall `Dockerfile.nightly` で再現可能なEnzyme環境を提供する
4. The CI system shall Enzyme テストの結果を専用レポートで出力する
5. Where Enzyme 環境が利用不可の場合, the build system shall graceful degradation を行う
