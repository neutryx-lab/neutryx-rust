# Requirements Document

## Introduction

AAD（随伴アルゴリズム微分）をサポートするイールドカーブ・ブートストラッパーの実装要件を定義する。本機能は `pricer_optimiser` (L2.5) クレートに配置され、市場データから割引カーブを構築し、Enzyme ADを通じて入力レートに対するカーブ感応度を効率的に計算する。

Tier-1銀行向けデリバティブ・プライシング・ライブラリとして、マルチカーブ・フレームワーク（OIS割引 + テナーカーブ）、複数補間方式、および実務レベルのエラー処理をサポートする。

## Requirements

### Requirement 1: 市場商品入力

**Objective:** As a クオンツ開発者, I want 複数種類の市場商品からカーブをブートストラップしたい, so that 実際の市場慣行に従ったカーブ構築が可能になる

#### Acceptance Criteria 1

1. When OISレートが入力された場合, the Bootstrapper shall 対応する満期のディスカウント・ファクターを計算する
2. When 金利スワップ（IRS）レートが入力された場合, the Bootstrapper shall スワップの固定レグとフローティングレグを等価にするディスカウント・ファクターを求解する
3. When FRA（Forward Rate Agreement）レートが入力された場合, the Bootstrapper shall 対応するフォワードレートを含むカーブを構築する
4. When 金利先物価格が入力された場合, the Bootstrapper shall コンベクシティ調整を適用してフォワードレートに変換する
5. If 入力商品の満期が重複している場合, then the Bootstrapper shall エラーを返し重複する満期を報告する
6. The Bootstrapper shall 少なくとも50年までの満期をサポートする

### Requirement 2: カーブ構築アルゴリズム

**Objective:** As a クオンツ開発者, I want 数値的に安定したブートストラップ・アルゴリズム, so that 任意の市場環境で信頼性の高いカーブを構築できる

#### Acceptance Criteria 2

1. The Bootstrapper shall 逐次ブートストラップ法（短期から長期へ順次解く方式）を実装する
2. When ブートストラップが完了した場合, the Bootstrapper shall 入力レートを1bp以内の精度で再現する
3. The Bootstrapper shall ディスカウント・ファクター、ゼロレート、フォワードレートの相互変換を提供する
4. While ブートストラップ実行中, the Bootstrapper shall 中間結果を保持し部分的なカーブ構築を可能にする
5. If Newton-Raphson法が収束しない場合, then the Bootstrapper shall Brent法にフォールバックする
6. The Bootstrapper shall 収束判定の許容誤差を設定可能とする（デフォルト: 1e-12）

### Requirement 3: AAD（随伴アルゴリズム微分）サポート

**Objective:** As a リスク管理者, I want カーブ感応度を効率的に計算したい, so that 大規模ポートフォリオのリスク計算を高速化できる

#### Acceptance Criteria 3

1. The Bootstrapper shall すべての入力レートに対するディスカウント・ファクターの偏微分を計算する
2. When AADモードが有効な場合, the Bootstrapper shall Enzyme ADを使用して勾配を計算する
3. The Bootstrapper shall バンプ・アンド・リバリュー方式との検証用比較機能を提供する
4. While AAD計算中, the Bootstrapper shall 不連続点のないスムース近似関数を使用する
5. The Bootstrapper shall num-dualモードとEnzymeモードの両方をフィーチャーフラグで切り替え可能とする
6. When 100個の入力レートがある場合, the Bootstrapper shall AADによりO(1)倍のコストで全感応度を計算する（バンプ方式のO(N)倍と比較）

### Requirement 4: 補間方式

**Objective:** As a クオンツ開発者, I want 複数の補間方式を選択したい, so that 市場慣行やモデル要件に応じた柔軟性を持てる

#### Acceptance Criteria 4

1. The Bootstrapper shall 線形補間（ゼロレートまたはログ・ディスカウント・ファクター）をサポートする
2. The Bootstrapper shall 三次スプライン補間をサポートする
3. The Bootstrapper shall モノトニック・キュービック補間をサポートする
4. The Bootstrapper shall フラット・フォワード補間をサポートする
5. When 補間方式が指定されない場合, the Bootstrapper shall デフォルトでログ線形補間を使用する
6. The Bootstrapper shall すべての補間方式がAAD互換であることを保証する

### Requirement 5: マルチカーブ・フレームワーク

**Objective:** As a 金利トレーダー, I want OIS割引とテナー別フォワードカーブを分離したい, so that 現代の金利市場慣行に準拠した評価ができる

#### Acceptance Criteria 5

1. The Bootstrapper shall OIS割引カーブの単独構築をサポートする
2. The Bootstrapper shall OIS割引を使用したIBOR/RFRテナーカーブの構築をサポートする
3. When マルチカーブモードの場合, the Bootstrapper shall 割引カーブとフォワードカーブの依存関係を正しく処理する
4. The Bootstrapper shall 3M、6Mなど複数テナーのカーブを同時に構築可能とする
5. If 割引カーブが提供されない場合, then the Bootstrapper shall 単一カーブ（自己割引）モードで動作する

### Requirement 6: 日付計算とカレンダー

**Objective:** As a クオンツ開発者, I want 正確な日付計算とカレンダー処理, so that 実務で使用されるカーブと整合する結果を得られる

#### Acceptance Criteria 6

1. The Bootstrapper shall ACT/360、ACT/365、30/360等の主要なデイカウント慣行をサポートする
2. The Bootstrapper shall 営業日調整（Following、Modified Following、Preceding）をサポートする
3. When ホリデーカレンダーが指定された場合, the Bootstrapper shall 休日を考慮したキャッシュフロー日付を計算する
4. The Bootstrapper shall `infra_master`のカレンダー機能と統合する
5. The Bootstrapper shall スポット日（T+2等）の計算をサポートする

### Requirement 7: エラー処理と検証

**Objective:** As a 運用担当者, I want 明確なエラーメッセージと検証機能, so that 問題発生時に迅速に対処できる

#### Acceptance Criteria 7

1. If 入力レートに負の値が含まれる場合, then the Bootstrapper shall 警告を出力し処理を継続するオプションを提供する
2. If ブートストラップが収束しない場合, then the Bootstrapper shall 収束に失敗した満期点と残差を報告する
3. When カーブ構築が完了した場合, the Bootstrapper shall 入力レートの再現精度を検証する機能を提供する
4. The Bootstrapper shall 構造化されたエラー型（`BootstrapError`）を使用する
5. If 入力データが不足している場合, then the Bootstrapper shall 必要な最小限の入力を明示するエラーを返す
6. The Bootstrapper shall カーブのアービトラージ・フリー条件（ディスカウント・ファクターの単調減少）を検証する

### Requirement 8: パフォーマンスとメモリ効率

**Objective:** As a システム・アーキテクト, I want 高性能かつメモリ効率の良い実装, so that 大規模な本番環境で使用できる

#### Acceptance Criteria 8

1. The Bootstrapper shall 100ポイントのカーブを10ミリ秒以内にブートストラップする
2. The Bootstrapper shall AAD計算時に動的メモリアロケーションを最小化する
3. The Bootstrapper shall スタティック・ディスパッチ（enumベース）を使用しEnzyme最適化と互換性を保つ
4. When 複数カーブをビルドする場合, the Bootstrapper shall 並列処理（Rayon）をサポートする
5. The Bootstrapper shall 中間計算結果のキャッシュ機能を提供する

### Requirement 9: 既存インフラとの統合

**Objective:** As a 開発者, I want 既存のNeutryxインフラとシームレスに統合したい, so that 一貫したコードベースを維持できる

#### Acceptance Criteria 9

1. The Bootstrapper shall `pricer_core::market_data::curves`の`YieldCurve`トレイトを実装する
2. The Bootstrapper shall `pricer_optimiser::solvers`の既存ソルバーを再利用する
3. The Bootstrapper shall `pricer_models::schedules`のスケジュール生成機能と統合する
4. When 構築されたカーブは, the Bootstrapper shall `PricingContext`で直接使用可能な形式で返す
5. The Bootstrapper shall `Float`トレイト境界を使用しジェネリックな数値型をサポートする
