# Requirements Document

## Introduction

本仕様は、`infra_master::trade::instrument`モジュールを拡充し、Tier-1銀行のトレーディングデスクで使用される定型金融商品を全資産クラス（Rates、FX、Equity、Credit、Commodity）にわたって包括的に定義することを目的とする。

各商品定義は、既存の`Trade` → `Vec<Leg>` → `Vec<Cashflow>`（CF展開）アーキテクチャと統合され、価格計算・リスク管理パイプラインで利用可能となる。

## Requirements

### Requirement 1: 金利商品（Rates）の拡充

**Objective:** As a クオンツ開発者, I want 金利デリバティブの定型商品を包括的に定義したい, so that イールドカーブ構築・金利リスク管理・価格計算を一貫して行える。

#### Acceptance Criteria

1. When Swaption商品が定義された場合, the instrument module shall 原資産スワップのテナー、行使日、行使タイプ（European/Bermudan/American）、決済タイプ（Cash/Physical）を表現できる。
2. When Cap/Floor商品が定義された場合, the instrument module shall ストライク、原資産インデックス、支払い頻度、ノーショナルスケジュールを表現できる。
3. When 変動金利債（FRN）が定義された場合, the instrument module shall クーポンインデックス、スプレッド、リセット頻度、元本償還スケジュールを表現できる。
4. When CMSスワップが定義された場合, the instrument module shall CMS参照テナー、コンベクシティ調整パラメータを表現できる。
5. When インフレスワップが定義された場合, the instrument module shall インフレインデックス（CPI等）、ラグ期間、ゼロクーポン/年次支払いタイプを表現できる。
6. The instrument module shall 既存のDeposit、Fra、Futures、ParSwap、Ois、BasisSwapとの整合性を維持する。

### Requirement 2: 為替商品（FX）の定義

**Objective:** As a FXトレーダー, I want FXデリバティブの定型商品を定義したい, so that FXオプション・フォワード・スワップの価格計算とリスク管理ができる。

#### Acceptance Criteria

1. When FXスポット取引が定義された場合, the instrument module shall 通貨ペア、スポットレート、決済日を表現できる。
2. When FXフォワード取引が定義された場合, the instrument module shall 通貨ペア、フォワードレート、決済日、ノーショナルを表現できる。
3. When FXバニラオプションが定義された場合, the instrument module shall 通貨ペア、ストライク、満期、コール/プット、ヨーロピアン/アメリカン行使タイプを表現できる。
4. When FXバリアオプションが定義された場合, the instrument module shall バリアレベル、バリアタイプ（Up/Down、In/Out）、ノックイン/ノックアウト条件を表現できる。
5. When FXスワップ（短期スワップ）が定義された場合, the instrument module shall ニアレグとファーレグの日付、レートを表現できる。
6. When クロスカレンシースワップが定義された場合, the instrument module shall 両通貨の元本、金利タイプ（固定/変動）、元本交換の有無を表現できる。

### Requirement 3: 株式商品（Equity）の定義

**Objective:** As a 株式デリバティブトレーダー, I want エクイティデリバティブの定型商品を定義したい, so that 株式オプション・フォワード・エキゾチック商品の価格計算ができる。

#### Acceptance Criteria

1. When 株式フォワードが定義された場合, the instrument module shall 原資産（単一株式/インデックス）、フォワード価格、決済日を表現できる。
2. When 株式バニラオプションが定義された場合, the instrument module shall 原資産、ストライク、満期、コール/プット、行使タイプを表現できる。
3. When 株式バリアオプションが定義された場合, the instrument module shall バリアレベル、モニタリング頻度（連続/離散）、バリアタイプを表現できる。
4. When アジアンオプションが定義された場合, the instrument module shall 平均タイプ（算術/幾何）、観測頻度、既存の観測値を表現できる。
5. When ルックバックオプションが定義された場合, the instrument module shall 固定ストライク/変動ストライクタイプ、観測期間を表現できる。
6. When エクイティスワップが定義された場合, the instrument module shall エクイティレグ（リターンタイプ：価格/トータルリターン）と金利レグを表現できる。
7. When バスケットオプションが定義された場合, the instrument module shall 構成銘柄、ウェイト、相関行列参照を表現できる。

### Requirement 4: クレジット商品（Credit）の定義

**Objective:** As a クレジットトレーダー, I want クレジットデリバティブの定型商品を定義したい, so that CDS・CDX等のクレジットリスク管理ができる。

#### Acceptance Criteria

1. When シングルネームCDSが定義された場合, the instrument module shall 参照エンティティ、想定元本、スプレッド、満期、リカバリーレートを表現できる。
2. When CDSインデックス（CDX/iTraxx）が定義された場合, the instrument module shall インデックス名、シリーズ、バージョン、構成銘柄数を表現できる。
3. When CDSオプション（CDSスワプション）が定義された場合, the instrument module shall 原資産CDS、ストライクスプレッド、行使日を表現できる。
4. When NTDバスケット（Nth-to-Default）が定義された場合, the instrument module shall バスケット構成、N番目パラメータ、相関パラメータを表現できる。
5. The instrument module shall ISDA標準のクレジットイベント定義（Bankruptcy、Failure to Pay、Restructuring等）を参照できる。

### Requirement 5: コモディティ商品（Commodity）の定義

**Objective:** As a コモディティトレーダー, I want コモディティデリバティブの定型商品を定義したい, so that エネルギー・金属・農産物の価格計算とリスク管理ができる。

#### Acceptance Criteria

1. When コモディティフォワードが定義された場合, the instrument module shall 原資産コモディティ、受渡場所、受渡日、数量単位を表現できる。
2. When コモディティスワップが定義された場合, the instrument module shall 固定価格レグと変動価格レグ（インデックス参照）を表現できる。
3. When コモディティバニラオプションが定義された場合, the instrument module shall 原資産、ストライク、満期、決済タイプ（現金/現物）を表現できる。
4. When コモディティアジアンオプションが定義された場合, the instrument module shall 価格平均期間、観測頻度を表現できる。
5. When スプレッドオプションが定義された場合, the instrument module shall 2つの原資産コモディティとスプレッドストライクを表現できる。
6. The instrument module shall コモディティタイプ（Energy、Metals、Agriculture）とサブタイプを分類できる。

### Requirement 6: CF展開（Trade変換）機能

**Objective:** As a 価格計算エンジン開発者, I want 定型商品をCF展開されたTrade構造に変換したい, so that 統一されたキャッシュフローベースの価格計算パイプラインで処理できる。

#### Acceptance Criteria

1. When 金利商品のCF展開が要求された場合, the instrument module shall `InstrumentDefinition`から`Trade`（`Vec<Leg>` → `Vec<Cashflow>`）への変換関数を提供する。
2. When FX商品のCF展開が要求された場合, the instrument module shall 元本交換キャッシュフローを含む`Trade`を生成する。
3. When オプション商品のCF展開が要求された場合, the instrument module shall 行使日に条件付きペイオフを持つ`Cashflow`を生成する。
4. When エキゾチック商品のCF展開が要求された場合, the instrument module shall 経路依存ペイオフを表現する拡張`Payoff`バリアントを使用する。
5. If CF展開に必要な市場慣行データが不足している場合, then the instrument module shall `InstrumentError`でエラー内容を明示する。
6. The instrument module shall CF展開の結果が既存の`Trade::all_cashflows()`メソッドで正しく列挙されることを保証する。
7. The instrument module shall `infra_master::convention`モジュールの市場慣行定義を活用してCF展開を行う。

### Requirement 7: 商品定義のデータ構造

**Objective:** As a システム開発者, I want 商品定義を型安全かつ拡張可能なデータ構造で表現したい, so that コンパイル時の型チェックと将来の商品追加が容易になる。

#### Acceptance Criteria

1. The instrument module shall 全商品定義を`InstrumentDefinition`列挙型で統一的に表現する。
2. The instrument module shall 各商品バリアントに必要最小限のフィールドのみを含める（過度な一般化を避ける）。
3. Where serde feature が有効な場合, the instrument module shall 全商品定義の`Serialize`/`Deserialize`を提供する。
4. The instrument module shall 商品定義のバリデーション関数を提供し、不正なパラメータ組み合わせを検出する。
5. The instrument module shall 商品タイプを問い合わせるヘルパーメソッド（`is_option()`, `is_swap()`, `asset_class()`等）を提供する。
6. While 新商品を追加する場合, the instrument module shall 既存のパターンに従い、破壊的変更なしに拡張可能な設計とする。

### Requirement 8: テストと検証

**Objective:** As a 品質管理担当者, I want 商品定義の正確性を検証したい, so that 価格計算の入力データが正しいことを保証できる。

#### Acceptance Criteria

1. The instrument module shall 各商品タイプに対して単体テストを提供する。
2. The instrument module shall CF展開の結果が期待されるキャッシュフロー列と一致することを検証するテストを提供する。
3. The instrument module shall 境界値（ゼロノーショナル、同一日の開始/終了等）に対するエッジケーステストを提供する。
4. If 無効なパラメータで商品が作成された場合, then the instrument module shall 適切なエラーを返すことをテストで検証する。
5. The instrument module shall proptestによるプロパティベーステストで、往復変換（商品→CF→商品）の一貫性を検証する。
