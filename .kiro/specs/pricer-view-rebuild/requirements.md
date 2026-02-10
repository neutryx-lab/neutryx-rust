# Requirements Document

## Project Description (Input)
Rebuild the PricerView in demo/gui from scratch. The current PricerView.vue (~1050 lines) is a monolithic component with 40+ refs, duplicated types/utilities, and no component decomposition. Goal: decompose into 13 sub-components, 4 composables, 1 Pinia store, and 1 constants file. Phased approach: Phase 1 (MVP: instrument select → params → expand → price → PV display), Phase 2 (Greeks, metrics, model/curve selection), Phase 3 (history, comparison), Phase 4 (polish, accessibility, export). Tech stack: Vue 3 + Composition API, Pinia, Tailwind CSS, TypeScript. Reuse existing services/api.ts, types/api.ts, utils/format.ts. Backend (Rust/Axum) endpoints already complete - no backend changes needed for Phase 1.

## Introduction

本ドキュメントは demo/gui の Pricer 画面をゼロベースでリビルドするための要件を定義する。現行の PricerView.vue（約1050行）はモノリシック構造、型定義の重複、APIサービスの未使用、40以上のリアクティブ参照のフラットな並列等の構造的問題を抱えている。本リビルドでは、コンポーネント分割・状態管理の一元化・既存インフラの再利用を通じて、保守性・拡張性・ユーザー体験を向上させる。

## Requirements

### Requirement 1: 商品選択とパラメータ入力

**Objective:** As a トレーダー/クオンツ, I want 商品を選択し、必要なパラメータを入力したい, so that 対象となるデリバティブ取引を正確に定義できる

#### Acceptance Criteria

1. When Pricer 画面がマウントされたとき, the PricerView shall `/api/instruments` から商品一覧を取得し、アセットクラスごとにグループ化したドロップダウンを表示する
2. When 商品が選択されたとき, the InstrumentSelector shall 当該商品の `requiredParams` に基づく動的パラメータフォームを生成する（number, date, select, text フィールド対応）
3. When IRS 商品が商品一覧に含まれる場合, the PricerView shall IRS を自動選択し、USD OIS 5Y のデフォルトパラメータ（notional: 1,000,000、currency: USD、fixedRate: 0.04、期間: 5年）を設定する
4. While 必須パラメータが未入力または不正な場合, the InstrumentSelector shall 対象フィールドにバリデーションエラーを表示し、Expand ボタンを無効化する
5. When 商品の選択が変更されたとき, the PricerView shall パラメータ・展開済み取引・計算結果をすべてリセットする

### Requirement 2: キャッシュフロー展開と表示

**Objective:** As a トレーダー/クオンツ, I want 取引のキャッシュフローを展開・確認したい, so that 取引構造を視覚的に把握できる

#### Acceptance Criteria

1. When 「Expand Cashflows」ボタンがクリックされたとき, the PricerView shall パラメータバリデーションを実行した後、`/api/trade/expand` にリクエストを送信し、展開結果をレグ別テーブルに表示する
2. While キャッシュフロー展開が進行中のとき, the CashflowTable shall ローディングスケルトンアニメーションを表示する
3. While キャッシュフローが未展開のとき, the CashflowTable shall 空状態のプレースホルダーメッセージを表示する
4. The CashflowTable shall 各レグについて、方向（Payer/Receiver）、通貨、レグタイプ、レートインデックスのバッジを表示する
5. The CashflowTable shall 各キャッシュフロー行について、支払日、発生期間、年数分、想定元本、レート、ペイオフタイプ、ディスカウントファクター（計算後）、PV（計算後）を表示する
6. The CashflowTable shall 展開メタデータ（レグ数、キャッシュフロー数、処理時間）をフッターに表示する

### Requirement 3: キャッシュフロー編集

**Objective:** As a クオンツ, I want キャッシュフローの想定元本やレートを手動で編集したい, so that What-if シナリオを即座に検証できる

#### Acceptance Criteria

1. The CashflowTable shall 各キャッシュフローの想定元本フィールドをインライン編集可能にする（コンパクト数値フォーマット K/M/B 対応）
2. The CashflowTable shall 固定レートのキャッシュフローについて、レートフィールドをインライン編集可能にする（パーセント表示）
3. While キャッシュフローが編集されている場合, the CashflowTable shall 編集済みセルをハイライト表示し、「Modified」インジケータと「Reset Edits」ボタンを表示する
4. When 「Reset Edits」ボタンがクリックされたとき, the CashflowTable shall すべての編集を取り消し、元のキャッシュフロー値に戻す
5. When 価格計算が実行されたとき, the PricerView shall 編集済みキャッシュフローの値を反映した金額でプライシングリクエストを構築する

### Requirement 4: 評価設定

**Objective:** As a クオンツ, I want 評価日・通貨・モデル設定・バンプサイズを設定したい, so that 価格計算の条件を柔軟に制御できる

#### Acceptance Criteria

1. The ValuationSettings shall 評価日（デフォルト: 当日）を日付ピッカーで設定可能にする
2. The ValuationSettings shall レポーティング通貨（USD, EUR, GBP, JPY）を選択可能にする
3. The ValuationSettings shall 「Use Default Model Config」トグルを提供し、無効時にパス数・ステップ数のカスタム設定を表示する
4. The ValuationSettings shall レートバンプ（bp）、FXバンプ（%）、ボルバンプ（%）のバンプサイズ設定を提供する

### Requirement 5: マーケットデータ連携

**Objective:** As a クオンツ, I want ディスカウントカーブと確率モデルを選択したい, so that 適切なマーケットデータとモデルを用いて価格計算を行える

#### Acceptance Criteria

1. The MarketDataSelector shall ディスカウントカーブ（USD-SOFR, EUR-ESTR, JPY-TONA, GBP-SONIA）をドロップダウンで選択可能にする
2. The ModelSelector shall 確率モデルタイプ（GBM, Heston, Hull-White, CIR）をドロップダウンで選択可能にする
3. When 確率モデルが変更されたとき, the ModelSelector shall 選択されたモデルのパラメータフォーム（各パラメータに min/max/step バリデーション付き）を動的に生成し、デフォルト値を設定する

### Requirement 6: 価格計算と PV 表示

**Objective:** As a トレーダー/クオンツ, I want 取引のPV（現在価値）を計算し結果を確認したい, so that 取引の経済的価値を定量的に評価できる

#### Acceptance Criteria

1. When 「Price & Risks」ボタンがクリックされたとき, the PricerView shall `/api/pricer/price` と `/api/pricer/greeks` に並列リクエストを送信する
2. While 展開済み取引が存在しない場合, the PricerActions shall 「Price & Risks」ボタンを無効化する
3. While 価格計算が進行中のとき, the PricerActions shall ボタンにスピナーアイコンと「Calculating...」テキストを表示する
4. The PvDisplay shall トータル PV を色分け（正値: success、負値: danger）で大きく表示する
5. The PvDisplay shall レグ別の PV 内訳を方向・通貨ラベル付きで表示する
6. Where 複数通貨のレグが存在する場合, the PvDisplay shall 通貨別の PV 集約を表示する

### Requirement 7: Greeks 計算と表示

**Objective:** As a リスクマネージャー, I want デルタ・ガンマ・シータ・ベガ等の Greeks を確認したい, so that リスクエクスポージャーを把握できる

#### Acceptance Criteria

1. The GreeksDisplay shall DV01（デルタ）、ガンマ、シータ、ベガを 2x2 グリッドで表示する
2. The GreeksDisplay shall 正値/負値に応じた色分けを適用する
3. If Greeks 計算が失敗した場合, the PricerView shall 警告通知を表示し、PV 結果は有効として保持する

### Requirement 8: 計算メトリクス

**Objective:** As a クオンツ, I want 計算のパフォーマンスメトリクスを確認したい, so that 計算効率をモニターできる

#### Acceptance Criteria

1. When 価格計算が完了したとき, the ComputationMetrics shall 処理時間（ミリ秒）、使用モデル、タイムスタンプを表示する

### Requirement 9: サマリー統計

**Objective:** As a トレーダー, I want 主要指標をひと目で確認したい, so that 現在の評価状況を素早く把握できる

#### Acceptance Criteria

1. The PricerSummaryBar shall 評価日・商品名・PV・DV01 の 4 つのサマリーカードを画面上部に常時表示する
2. While 価格計算結果が存在しない場合, the PricerSummaryBar shall PV・DV01 カードにプレースホルダー「-」を表示する

### Requirement 10: 結果履歴と比較

**Objective:** As a クオンツ, I want 過去の計算結果を保持し比較したい, so that パラメータ変更の影響を分析できる

#### Acceptance Criteria

1. When 価格計算が成功したとき, the PricerView shall 結果（パラメータ・PV・Greeks・設定情報）を履歴に自動追加する（最大 5 件保持）
2. The PricerHistory shall 直近の履歴エントリを商品名・タイムスタンプ・PV 付きのリストで表示する
3. When 履歴エントリがクリックされたとき, the PricerHistory shall 当該エントリのパラメータと結果を画面に復元する
4. When 前回の結果が存在する場合, the PvDisplay shall 現在の PV と前回の PV の差分（絶対値・パーセント）を表示する
5. Where 履歴に 2 件以上のエントリが存在する場合, the PricerHistory shall 比較モードトグルを表示し、2 件の結果を並べて PV・変更パラメータを比較表示する

### Requirement 11: エラーハンドリングとフィードバック

**Objective:** As a ユーザー, I want 操作結果や異常を明確に通知されたい, so that 次のアクションを適切に判断できる

#### Acceptance Criteria

1. If API リクエストが失敗した場合, the PricerView shall Toast 通知でエラーメッセージを表示する
2. If 商品一覧の取得に失敗した場合, the PricerView shall API 利用不可メッセージを表示し、プライシング機能を無効化する
3. When 価格計算が成功したとき, the PricerView shall 成功 Toast 通知を表示する
4. When バリデーションエラーが存在する場合, the PricerView shall 警告 Toast 通知を表示する

### Requirement 12: コンポーネント構造（非機能）

**Objective:** As a 開発者, I want モジュラーなコンポーネント構造で画面を構成したい, so that 保守性・テスト容易性・再利用性を確保できる

#### Acceptance Criteria

1. The PricerView shall サブコンポーネントの合成によるオーケストレータとして機能し、200 行以内に収める
2. The PricerView shall Pinia ストア（`stores/pricer.ts`）に全 Pricer 状態を一元管理する
3. The PricerView shall 既存の `services/api.ts` の API 関数（`fetchInstruments`, `expandTrade`, `priceTrade`, `calculateGreeks`）を使用し、raw `fetch()` 呼び出しを行わない
4. The PricerView shall 既存の `types/api.ts` の型定義を使用し、ローカルなインターフェース再宣言を行わない
5. The PricerView shall 既存の `utils/format.ts` のフォーマット関数（`formatCurrency`, `formatNumberCompact`, `parseFormattedNumber`）を使用し、ユーティリティの重複定義を行わない
6. The PricerView shall 定数（`STOCHASTIC_MODELS`, `CURVE_OPTIONS`）を `constants/pricer.ts` に抽出する

### Requirement 13: レイアウトと視覚デザイン

**Objective:** As a ユーザー, I want 整理された直感的なレイアウトで作業したい, so that 効率的にプライシング操作を行える

#### Acceptance Criteria

1. The PricerView shall 3 カラムグリッド（左: 設定パネル 1/3、右: キャッシュフロー 2/3）のレイアウトを採用する
2. The PricerView shall 既存のダッシュボードデザインシステム（`glass-card` スタイル、CSS 変数 `--surface`, `--primary`, `--text-primary` 等）に準拠する
3. While API が利用不可の場合, the PricerView shall 設定パネル・キャッシュフローテーブルの代わりに、API 利用不可のフォールバック画面を表示する
