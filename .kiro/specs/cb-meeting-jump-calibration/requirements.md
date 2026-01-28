# Requirements Document

## Introduction

本仕様は、demo_guiのCurveBuilderにおけるGlobalSolverに中央銀行会合(CB Meeting)日のフォワードレートジャンプを考慮したカリブレーション機能を追加するものである。Eventの入力値に期待ジャンプ幅を追加し、GlobalSolverがCB Meeting日におけるフォワードレートの不連続性を正確にモデル化できるようにする。

### 背景

金利カーブのカリブレーションにおいて、中央銀行の政策金利決定会合（FOMC、ECB会合、日銀金融政策決定会合等）の前後でフォワードレートがジャンプする現象は市場で広く認識されている。現行のGlobalBootstrapperはこのジャンプを考慮せず、滑らかなカーブを生成するため、CB Meeting日付近のプライシング精度に影響がある。

### 用語定義

- **CB Meeting**: 中央銀行の金融政策決定会合（FED FOMC、ECB Governing Council、BOJ金融政策決定会合等）
- **期待ジャンプ幅**: CB Meeting日におけるフォワードレートの予想変化幅（basis points）
- **GlobalSolver**: `pricer_models::builder::curve::global::GlobalBootstrapper`による多次元Newton-Raphson法を用いたカリブレーション手法

---

## Requirements

### Requirement 1: Event期待ジャンプ幅の入力拡張

**Objective:** As a クオンツ開発者, I want CB Meeting Eventに期待ジャンプ幅を入力できるようにする, so that カーブカリブレーションで政策金利変更の市場予想を反映できる

#### Acceptance Criteria

1. When ユーザーがCB Meeting Eventを選択する, the CurveBuilder shall 期待ジャンプ幅(bps)の入力フィールドを表示する
2. The MarketEvent shall 期待ジャンプ幅を保持するオプショナルフィールド`expected_jump_bps`を持つ
3. When 期待ジャンプ幅が入力されていない, the CurveBuilder shall デフォルト値0を使用する
4. If 期待ジャンプ幅が-100bpsから+100bpsの範囲外, then the CurveBuilder shall バリデーションエラーを表示する
5. The API shall CB Meeting日付と期待ジャンプ幅のペアをJSON形式で受け取る

---

### Requirement 2: GlobalSolverのジャンプ対応

**Objective:** As a クオンツ開発者, I want GlobalSolverがCB Meeting日のジャンプを考慮してカリブレーションを行う, so that 政策金利決定日前後のフォワードレート不連続性を正確にモデル化できる

#### Acceptance Criteria

1. When CB Meetingイベントが指定されている, the GlobalBootstrapper shall 該当日付をジャンプピラーとしてカリブレーショングリッドに追加する
2. The GlobalBootstrapper shall ジャンプピラーにおいてフォワードレートの不連続を許容するよう補間ロジックを調整する
3. When 商品のキャッシュフローがCB Meeting日を跨ぐ, the CalibrationInstrument shall ジャンプ調整後のディスカウントファクターを使用してtheoretical_rateを計算する
4. The GlobalBootstrapResult shall ジャンプピラーの情報と各ジャンプの実現値を含む
5. While ジャンプカリブレーションが有効, the GlobalBootstrapper shall Jacobian行列にジャンプパラメータの偏微分を含める

---

### Requirement 3: CashflowMatrixへのジャンプ統合

**Objective:** As a クオンツ開発者, I want CashflowMatrixがCB Meetingジャンプを表現できる, so that 既存のカリブレーションインフラを活用しつつジャンプを統合できる

#### Acceptance Criteria

1. The CalibrationMatrix shall ジャンプピラーを示す追加の列を持つ（jump_pillar_flags）
2. When ジャンプピラーが設定されている, the InterpolationMatrix shall 該当日付で補間重みを不連続にする
3. The CalibrationProblem shall ジャンプパラメータを含む拡張パラメータベクトルをサポートする
4. If ジャンプピラーが通常のピラーと一致する, then the GlobalBootstrapper shall 単一のピラーとして処理し重複を避ける
5. The GlobalTimeGrid shall ジャンプイベント日を自動的にグリッドに追加する

---

### Requirement 4: API拡張

**Objective:** As a フロントエンド開発者, I want REST APIがCB Meetingジャンプパラメータを受け付ける, so that WebUIからジャンプ対応カリブレーションを呼び出せる

#### Acceptance Criteria

1. The `/api/curves/build` endpoint shall `cb_events`オプショナルパラメータを受け付ける
2. When `cb_events`が指定されている, the API shall 各イベントの日付と期待ジャンプ幅をパースする
3. The API response shall カリブレーション後の実現ジャンプ値を含む
4. If CB Meeting日付が商品テナー範囲外, then the API shall 該当イベントを無視し警告をログに出力する
5. The API shall 複数通貨のCB Meetingを同時にサポートする

---

### Requirement 5: WebUI表示

**Objective:** As a トレーダー/リスク管理者, I want CurveBuilder UIでCB Meetingジャンプを視覚的に確認できる, so that カリブレーション結果の妥当性を直感的に判断できる

#### Acceptance Criteria

1. When CB Meetingジャンプが有効, the CurveBuilder UI shall フォワードカーブ上にジャンプ日付をマーカーで表示する
2. The CurveBuilder UI shall ジャンプ前後のフォワードレート値をツールチップで表示する
3. When ユーザーがジャンプマーカーをクリック, the CurveBuilder UI shall 該当CB Meetingの詳細情報（中央銀行名、日付、期待ジャンプ幅、実現ジャンプ幅）を表示する
4. The CurveBuilder UI shall ジャンプ有効/無効をトグルで切り替えられる
5. While ジャンプが有効, the CurveBuilder UI shall カーブ描画時に不連続点を適切に処理する（ギャップとして表示、または線で接続のオプション）

---

### Requirement 6: バリデーションとエラーハンドリング

**Objective:** As a システム管理者, I want ジャンプカリブレーションのエラーが適切に処理される, so that 不正な入力や収束失敗時にシステムが安定動作する

#### Acceptance Criteria

1. If CB Meeting日付が無効なフォーマット, then the CurveBuilder shall 明確なエラーメッセージを返す
2. If 期待ジャンプ幅が数値でない, then the CurveBuilder shall バリデーションエラーを返す
3. If ジャンプ付きカリブレーションが収束しない, then the GlobalBootstrapper shall ジャンプなしでの再カリブレーションを試み、ユーザーに警告を表示する
4. The CalibrationError shall ジャンプ関連のエラーバリアント（JumpCalibrationFailed, InvalidJumpParameter）を含む
5. When デバッグモードが有効, the GlobalBootstrapper shall 各イテレーションでのジャンプパラメータ値をログに出力する

---

### Requirement 7: 後方互換性

**Objective:** As a 既存ユーザー, I want 既存のカリブレーションワークフローが影響を受けない, so that ジャンプ機能を使わない場合でも従来通りシステムを利用できる

#### Acceptance Criteria

1. When CB Meetingイベントが指定されていない, the GlobalBootstrapper shall 従来と同一のカリブレーション結果を生成する
2. The API shall 新規パラメータをオプショナルとし、既存リクエストとの互換性を維持する
3. The MarketEvent shall 既存フィールドの変更なしにexpected_jump_bpsを追加する
4. While ジャンプ機能が無効, the CurveBuilder UI shall 従来のUI/UXを維持する
5. The GlobalBootstrapConfig shall ジャンプ機能のデフォルトを無効（false）とする

---

## Technical Notes

### 既存アーキテクチャとの整合性

- **MarketEvent**: `infra_master::market::events`に`expected_jump_bps: Option<f64>`を追加
- **GlobalBootstrapper**: `pricer_models::builder::curve::global`にジャンプ対応ロジックを追加
- **CalibrationMatrix**: `pricer_models::builder::matrix`にジャンプピラーフラグを追加
- **API Handler**: `demo/gui/src/web/handlers/curves.rs`にCB Meetingジャンプパラメータ処理を追加

### 参照ファイル

- `crates/infra_master/src/market/events/mod.rs` - MarketEvent定義
- `crates/pricer_models/src/builder/curve/global.rs` - GlobalBootstrapper
- `crates/pricer_models/src/builder/matrix.rs` - CalibrationMatrix
- `demo/gui/src/web/handlers/curves.rs` - REST APIハンドラ
- `demo/data/input/events/central_bank_meetings.json` - CB Meetingデータ
