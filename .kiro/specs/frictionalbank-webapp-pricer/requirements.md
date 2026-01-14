# Requirements Document

## Introduction

FrictionalBank WebAppにインタラクティブなプライサー機能を追加する。ユーザーがWebインターフェース上でデリバティブ商品を選択し、パラメータを入力してリアルタイムで価格計算とGreeks表示を行えるようにする。既存のFrictionalBank demo基盤（`demo/gui/web/`）を拡張し、pricer_pricing（L3）およびpricer_models（L2）の計算エンジンと統合する。

## Requirements

### Requirement 1: 商品選択インターフェース

**Objective:** As a トレーダー/クオンツアナリスト, I want to WebUI上でデリバティブ商品タイプを選択できること, so that 価格計算対象を明確に指定できる。

#### Acceptance Criteria 1

1. When ユーザーがプライサーページにアクセスした時, the WebApp shall 利用可能な商品タイプ一覧（Equity Vanilla Option、FX Option、IRS、CDS等）を表示する。
2. When ユーザーが商品タイプを選択した時, the WebApp shall 当該商品に適したパラメータ入力フォームを動的に表示する。
3. The WebApp shall 各商品タイプに対応するアイコンまたはラベルを表示する。

### Requirement 2: パラメータ入力フォーム

**Objective:** As a トレーダー, I want to 商品固有のパラメータ（満期、ストライク、想定元本等）を入力できること, so that 具体的な取引条件で価格計算を実行できる。

#### Acceptance Criteria 2

1. When Equity Vanilla Optionが選択された時, the WebApp shall スポット価格、ストライク、満期日、ボラティリティ、金利、オプションタイプ（Call/Put）の入力フィールドを表示する。
2. When FX Optionが選択された時, the WebApp shall 通貨ペア、スポットレート、ストライク、満期日、ドメスティック金利、フォーリン金利の入力フィールドを表示する。
3. When IRSが選択された時, the WebApp shall 想定元本、開始日、満期日、固定レート、変動レート参照、支払頻度の入力フィールドを表示する。
4. If 必須パラメータが未入力の場合, then the WebApp shall 該当フィールドをハイライトしエラーメッセージを表示する。
5. The WebApp shall 数値入力フィールドに適切なバリデーション（正数、日付形式等）を適用する。

### Requirement 3: リアルタイム価格計算

**Objective:** As a トレーダー, I want to 入力パラメータに基づいてリアルタイムで価格を計算できること, so that 取引判断を迅速に行える。

#### Acceptance Criteria 3

1. When ユーザーが「Calculate」ボタンをクリックした時, the WebApp shall バックエンドにWebSocket経由で価格計算リクエストを送信する。
2. When 価格計算が完了した時, the WebApp shall 計算結果（PV、Fair Value）を即座に画面に表示する。
3. While 価格計算が実行中の間, the WebApp shall ローディングインジケータを表示する。
4. If 価格計算でエラーが発生した場合, then the WebApp shall エラーメッセージを表示しユーザーに修正を促す。
5. The WebApp shall 計算結果を3桁区切りフォーマットで通貨単位付きで表示する。

### Requirement 4: Greeks表示

**Objective:** As a リスクマネージャー, I want to 計算されたGreeks（Delta、Gamma、Vega、Theta、Rho）を確認できること, so that ポジションのリスク特性を把握できる。

#### Acceptance Criteria 4

1. When 価格計算が完了した時, the WebApp shall Delta、Gamma、Vega、Theta、Rhoの値を表示する。
2. The WebApp shall 各Greekの値を適切な小数点桁数でフォーマットして表示する。
3. Where 高度な分析オプションが有効な場合, the WebApp shall 二次Greeks（Vanna、Volga等）も表示する。
4. The WebApp shall 正負のGreeks値を色分け（正：緑、負：赤）で視覚的に区別する。

### Requirement 5: バックエンドAPI統合

**Objective:** As a システム, I want to pricer_pricing/pricer_modelsと適切に統合されること, so that 正確な価格計算を提供できる。

#### Acceptance Criteria 5

1. The WebApp Backend shall pricer_models::InstrumentEnumを使用して商品を構築する。
2. The WebApp Backend shall pricer_pricing::contextを使用して価格計算を実行する。
3. The WebApp Backend shall Greeks計算にpricer_pricing::greeksモジュールを使用する。
4. When APIリクエストを受信した時, the WebApp Backend shall 入力パラメータをバリデーションしてからpricer層に渡す。
5. If pricer層でエラーが発生した場合, then the WebApp Backend shall 適切なエラーレスポンスをクライアントに返す。

### Requirement 6: 市場データ連携

**Objective:** As a トレーダー, I want to デフォルト市場データ（イールドカーブ、ボラティリティサーフェス）を使用できること, so that 手動入力の手間を軽減できる。

#### Acceptance Criteria 6

1. The WebApp shall デモ用のデフォルトイールドカーブを提供する。
2. The WebApp shall デモ用のデフォルトボラティリティサーフェスを提供する。
3. When ユーザーが「Use Market Data」オプションを選択した時, the WebApp shall デフォルト市場データをパラメータに適用する。
4. The WebApp shall 使用中の市場データソース（Demo/Custom）を明示的に表示する。

### Requirement 7: 計算履歴

**Objective:** As a クオンツアナリスト, I want to 過去の計算結果を参照できること, so that 異なるシナリオの比較分析ができる。

#### Acceptance Criteria 7

1. When 価格計算が完了した時, the WebApp shall 計算結果をセッション内履歴に追加する。
2. The WebApp shall 直近10件の計算履歴をサイドパネルに表示する。
3. When ユーザーが履歴項目をクリックした時, the WebApp shall 当該計算のパラメータと結果を復元表示する。
4. The WebApp shall 履歴項目に計算日時と商品タイプを表示する。

### Requirement 8: レスポンシブUI

**Objective:** As a エンドユーザー, I want to 様々なデバイスサイズで快適に操作できること, so that デスクトップでもタブレットでも利用可能である。

#### Acceptance Criteria 8

1. The WebApp shall デスクトップ（1920px幅以上）で最適なレイアウトを表示する。
2. The WebApp shall タブレット（768px〜1919px幅）でも操作可能なレイアウトを維持する。
3. The WebApp shall 入力フォームとGreeks表示を明確に視覚的に区分する。
4. The WebApp shall アクセシビリティ基準（WCAG 2.1 AA）に準拠したコントラスト比を維持する。
