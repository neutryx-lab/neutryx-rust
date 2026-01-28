# Implementation Plan: CB Meeting Jump Calibration

## Tasks

- [x] 1. MarketEventへの期待ジャンプ幅フィールド追加
- [x] 1.1 (P) MarketEvent構造体に`expected_jump_bps`フィールドを追加
  - CB Meetingイベントに期待ジャンプ幅（basis points）を保持するオプショナルフィールドを追加
  - Serdeの`#[serde(default, skip_serializing_if)]`属性で後方互換性を維持
  - 既存のイベント読み込みテストが引き続きパスすることを確認
  - _Requirements: 1.2, 7.3_
  - _Contracts: MarketEvent State_

- [x] 2. ジャンプピラーとジャンプ設定の構造体定義
- [x] 2.1 (P) JumpPillar構造体の実装
  - ジャンプ日付（年分数）、期待ジャンプ幅、実現ジャンプ幅、パラメータインデックスを保持
  - bps→absolute rate変換メソッド（0.0001倍）を実装
  - 日付文字列から年分数への変換コンストラクタを実装
  - _Requirements: 2.1_
  - _Contracts: JumpPillar State_

- [x] 2.2 (P) JumpConfig構造体の実装
  - ジャンプ機能有効/無効フラグ、ジャンプピラーリスト、フォールバック設定を保持
  - Builder patternでwith_*メソッドを提供
  - Default traitでデフォルト無効（enabled: false）を設定
  - _Requirements: 2.1, 7.5_
  - _Contracts: JumpConfig State_

- [x] 3. GlobalBootstrapConfigの拡張
- [x] 3.1 GlobalBootstrapConfigにジャンプ設定フィールドを追加
  - `jump_config: Option<JumpConfig<T>>`フィールドを追加
  - `with_jump_config`および`with_jumps`ビルダーメソッドを追加
  - 既存の設定フィールドとメソッドを変更しない
  - _Requirements: 7.5_
  - _Contracts: GlobalBootstrapConfig State_

- [x] 4. CalibrationErrorへのジャンプ関連エラーバリアント追加
- [x] 4.1 (P) ジャンプカリブレーション用エラーバリアントを追加
  - `JumpCalibrationFailed`バリアント（メッセージ、最終残差、イテレーション数）を追加
  - `InvalidJumpParameter`バリアント（日付、値、理由）を追加
  - 既存のエラーバリアントとの整合性を維持
  - _Requirements: 6.4_
  - _Contracts: CalibrationError State_

- [ ] 5. InterpolationMatrixのジャンプ対応補間拡張
- [ ] 5.1 ジャンプピラーを考慮した補間マトリックス生成
  - `with_jump_pillars`メソッドでジャンプ日付を補間区間の境界として扱う
  - ジャンプ前後で別々の補間セグメントを適用
  - 既存の`from_pillars`メソッドの動作を変更しない
  - _Requirements: 2.2, 3.2_
  - _Contracts: InterpolationMatrix Service_

- [ ] 5.2 ジャンプ調整付き補間メソッドの実装
  - `interpolate_with_jumps`メソッドでジャンプパラメータを適用
  - DF乗算方式（DF × Π(1 + jump_i)）で累積効果を計算
  - 補間重みとジャンプ調整を分離して計算
  - _Requirements: 2.2, 3.2_

- [ ] 6. CalibrationProblemのジャンプ対応拡張
- [ ] 6.1 拡張パラメータベクトルでの問題構築
  - `with_jumps`コンストラクタでジャンプピラーを受け取る
  - パラメータベクトルを`[log(DF_1), ..., log(DF_n), jump_1, ..., jump_m]`に拡張
  - ジャンプピラーのパラメータインデックスを設定
  - _Requirements: 3.3_
  - _Contracts: CalibrationProblem Service_

- [ ] 6.2 ジャンプ調整付きカーブ構築メソッドの実装
  - `build_curve_with_jumps`メソッドでジャンプを適用したディスカウントファクターを計算
  - 各ピラーのDFにジャンプ効果を乗算
  - BootstrappedCurveとして返却
  - _Requirements: 2.3_

- [ ] 6.3 ジャンプパラメータを含むJacobian計算の実装
  - `compute_jacobian_with_jumps`メソッドで拡張Jacobian行列を計算
  - ジャンプパラメータの偏微分を計算（∂F/∂jump）
  - 既存のFinite Difference/Central Difference手法を拡張
  - _Requirements: 2.5_

- [ ] 7. GlobalBootstrapperのジャンプ対応拡張
- [ ] 7.1 ジャンプピラーのグリッド統合
  - `merge_pillars`メソッドで通常ピラーとジャンプピラーをマージ
  - 重複ピラーを検出し単一ピラーとして処理
  - マージ後のピラー配列とジャンプインデックスを返却
  - _Requirements: 2.1, 3.4, 3.5_
  - _Contracts: GlobalBootstrapper Service_

- [ ] 7.2 ジャンプ付きカリブレーションメソッドの実装
  - `calibrate_with_jumps`メソッドで拡張Newton-Raphsonを実行
  - CalibrationProblem.with_jumpsを使用して問題を構築
  - 収束判定とイテレーション管理
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

- [ ] 7.3 フォールバック戦略の実装
  - `fallback_calibrate`メソッドでジャンプなしカリブレーションを実行
  - 収束失敗時に自動的にフォールバックを試行
  - フォールバック使用フラグと警告メッセージを設定
  - _Requirements: 6.3_

- [ ] 7.4 デバッグログの追加
  - debug_loggingが有効な場合、各イテレーションでジャンプパラメータ値を出力
  - 収束状況とJacobian条件数をログに記録
  - _Requirements: 6.5_

- [ ] 8. GlobalBootstrapResultのジャンプ情報拡張
- [ ] 8.1 カリブレーション結果にジャンプ情報を追加
  - `realized_jumps`フィールドで実現ジャンプ値（時間、bps）を格納
  - `fallback_used`フラグでフォールバック使用を示す
  - `jump_warnings`でジャンプ関連警告を格納
  - _Requirements: 2.4_
  - _Contracts: GlobalBootstrapResult State_

- [ ] 9. APIリクエスト/レスポンスの拡張
- [ ] 9.1 (P) CurveBuildRequestにCB Meetingパラメータを追加
  - `cb_events: Option<Vec<CbEventInput>>`フィールドを追加
  - `enable_jumps: bool`フラグを追加
  - Serdeのデフォルト属性で後方互換性を維持
  - _Requirements: 4.1, 4.2, 7.2_
  - _Contracts: CurveBuildRequest API_

- [ ] 9.2 (P) CbEventInput構造体の実装
  - 日付（ISO形式）、期待ジャンプ幅（bps）、中央銀行コードを保持
  - ±100bpsのバリデーション範囲を定義
  - camelCase JSON命名でフロントエンド互換
  - _Requirements: 1.5, 4.2_

- [ ] 9.3 (P) CurveBuildResponseに実現ジャンプ情報を追加
  - `realized_jumps: Option<Vec<RealizedJumpInfo>>`フィールドを追加
  - `jump_fallback_used`フラグを追加
  - `jump_warnings`配列を追加
  - _Requirements: 4.3_
  - _Contracts: CurveBuildResponse API_

- [ ] 10. APIハンドラのジャンプ対応
- [ ] 10.1 CB Meetingパラメータのパースとバリデーション
  - cb_eventsから日付と期待ジャンプ幅を抽出
  - 日付フォーマット検証（ISO 8601）
  - ジャンプ幅の範囲検証（±100bps）
  - 数値型検証
  - _Requirements: 1.3, 1.4, 6.1, 6.2_

- [ ] 10.2 JumpPillarへの変換と範囲フィルタリング
  - CbEventInputからJumpPillarへの変換ロジック
  - 商品テナー範囲外のイベントを除外し警告を記録
  - 複数通貨のイベントを同時処理
  - _Requirements: 4.4, 4.5_

- [ ] 10.3 GlobalBootstrapper呼び出しの統合
  - enable_jumpsフラグに基づきcalibrate_with_jumpsまたはcalibrateを呼び出し
  - 結果からrealized_jumpsを抽出しレスポンスに格納
  - フォールバック情報と警告をレスポンスに含める
  - _Requirements: 4.3_

- [ ] 11. WebUIのジャンプ入力機能
- [ ] 11.1 (P) CB Meetingイベント選択時のジャンプ入力フィールド表示
  - CB Meetingイベント選択でジャンプ幅入力フィールドを表示
  - bps単位のラベルと入力ヒントを表示
  - デフォルト値0で初期化
  - _Requirements: 1.1, 1.3_

- [ ] 11.2 (P) ジャンプ有効/無効トグルスイッチの実装
  - カリブレーション設定パネルにトグルを追加
  - トグル状態に応じてジャンプ入力フィールドの表示/非表示を切り替え
  - _Requirements: 5.4, 7.4_

- [ ] 12. WebUIのジャンプ可視化
- [ ] 12.1 フォワードカーブ上のジャンプマーカー表示
  - Chart.jsでジャンプ日付位置にマーカーを描画
  - マーカーの色とスタイルで視認性を確保
  - _Requirements: 5.1_

- [ ] 12.2 ジャンプマーカーのツールチップ表示
  - マーカーホバー時にジャンプ前後のフォワードレート値を表示
  - 期待ジャンプ幅と実現ジャンプ幅を表示
  - _Requirements: 5.2_

- [ ] 12.3 ジャンプマーカークリック時の詳細情報表示
  - クリックで詳細パネルを表示
  - 中央銀行名、日付、期待ジャンプ幅、実現ジャンプ幅を含める
  - _Requirements: 5.3_

- [ ] 12.4 不連続点のカーブ描画処理
  - ジャンプ日付でカーブを分割して描画
  - ギャップ表示または線接続のオプションを提供
  - _Requirements: 5.5_

- [ ] 13. 統合テストとエンドツーエンド検証
- [ ] 13.1 ジャンプ付きカリブレーションの統合テスト
  - 単一ジャンプ付きOISカリブレーションの検証
  - 複数ジャンプ付きカリブレーションの累積効果検証
  - ジャンプなし時の既存結果との同一性検証
  - _Requirements: 2.1, 7.1_

- [ ] 13.2 フォールバック動作の検証テスト
  - 意図的に収束失敗を発生させてフォールバック発動を確認
  - フォールバック結果に警告メッセージが含まれることを検証
  - _Requirements: 6.3_

- [ ] 13.3 API統合テスト
  - POST /api/curves/build with cb_eventsの正常系テスト
  - バリデーションエラー（日付不正、範囲外）のテスト
  - レスポンスにrealized_jumpsが含まれることを検証
  - _Requirements: 4.1, 4.2, 4.3, 6.1, 6.2_

- [ ]* 13.4 後方互換性テスト
  - 既存リクエスト（cb_eventsなし）が従来通り動作することを確認
  - 新規フィールドのデフォルト値が適切であることを検証
  - _Requirements: 7.1, 7.2, 7.3, 7.5_

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1.1 | 11.1 |
| 1.2 | 1.1 |
| 1.3 | 10.1, 11.1 |
| 1.4 | 10.1 |
| 1.5 | 9.2 |
| 2.1 | 2.1, 7.1, 7.2, 13.1 |
| 2.2 | 5.1, 5.2, 7.2 |
| 2.3 | 6.2, 7.2 |
| 2.4 | 7.2, 8.1 |
| 2.5 | 6.3, 7.2 |
| 3.2 | 5.1, 5.2 |
| 3.3 | 6.1 |
| 3.4 | 7.1 |
| 3.5 | 7.1 |
| 4.1 | 9.1, 13.3 |
| 4.2 | 9.1, 9.2, 13.3 |
| 4.3 | 9.3, 10.3, 13.3 |
| 4.4 | 10.2 |
| 4.5 | 10.2 |
| 5.1 | 12.1 |
| 5.2 | 12.2 |
| 5.3 | 12.3 |
| 5.4 | 11.2 |
| 5.5 | 12.4 |
| 6.1 | 10.1, 13.3 |
| 6.2 | 10.1, 13.3 |
| 6.3 | 7.3, 13.2 |
| 6.4 | 4.1 |
| 6.5 | 7.4 |
| 7.1 | 13.1, 13.4 |
| 7.2 | 9.1, 13.4 |
| 7.3 | 1.1, 13.4 |
| 7.4 | 11.2 |
| 7.5 | 2.2, 3.1, 13.4 |
