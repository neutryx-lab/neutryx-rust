# Requirements Document

## Project Description (Input)
「3. 取引定義（Human Definition）と価格評価（Pricing Representation）の分離」について、金融工学ライブラリにおける現代的かつ最も強力なアプローチである**「データ指向設計（Data-Oriented Design）への移行」**を提案します。

既存のオブジェクト指向的な「階層構造（Trade > Leg > Schedule > Coupon）」を、数値計算エンジンが好む「リニアな配列構造（Linear Arrays / Streams）」に変換する**コンパイル・フェーズ**を導入します。

以下にその具体的な構造と設計を示します。

---

### コンセプト：Pricing IR（中間表現）の導入

現在の「Swap 定義」は人間が読むための契約書に近いものです。これをCPUが処理しやすい「命令セット」に変換します。

1. **Source (Input):** `Trade` (階層的、日付、カレンダー、文字列)
2. **Compiler:** `TradeBuilder` (日付計算、休日調整、スケジュール展開を行う)
3. **IR (Output):** `PricingKernel` (完全に平坦化された `f64` と `usize` の配列)

### 具体的な構造案

プライシングエンジン（モンテカルロや解析解ソルバー）が受け取る構造体を、以下のような「SoA（Structure of Arrays）」スタイルに変更します。

#### 1. データ構造の定義（Rustコード例）

従来の `Vec<Box<dyn Instrument>>` や `enum Instrument` の代わりに、商品タイプを問わず共通して使える単一の構造体を定義します。

```rust
/// プライシング専用の中間表現 (Intermediate Representation)
/// 全ての金融商品をこの形式（またはその組み合わせ）に「コンパイル」します。
pub struct LinearCashflowEngine {
    // --- 時間軸 (Time Domain) ---
    // すべてのキャッシュフローが発生する時間 (年単位, t=0 is valuation date)
    // 昇順ソート済み
    pub flow_times: Vec<f64>,

    // --- 確定キャッシュフロー (Fixed Flows) ---
    // flow_times に対応する確定額 (固定金利 * Notional * YearFraction)
    // 発生しないタイムステップは 0.0
    pub fixed_amounts: Vec<f64>,

    // --- 確率的キャッシュフロー (Floating Flows) ---
    // 参照するリスクファクターのインデックス (例: 0=USD-SOFR, 1=EUR-ESTR)
    pub index_pointers: Vec<usize>,

    // 変動部分の乗数 (Notional * YearFraction)
    pub gearing: Vec<f64>,

    // スプレッド部分 (Spread * Notional * YearFraction)
    pub spreads: Vec<f64>,

    // --- 割引 (Discounting) ---
    // 割引に使用するカーブのインデックス
    pub discount_curve_ids: Vec<usize>,
}
```

#### 2. この構造が「強力」である理由

この構造に変更することで、以下のような劇的なメリットが生まれます。

**A. 条件分岐の排除 (Branchless Logic)**
従来の `match instrument` では、商品ごとに異なるコードパスが実行され、CPUのパイプラインハザードが発生していました。
上記の構造であれば、エンジンは商品の種類（Swap, Bond, FRA）を知る必要がありません。単に配列を上から下へ計算するだけです。

```rust
// エンジンのメインループ（擬似コード）
// もはや商品ごとの分岐は存在しない
for i in 0..flow_times.len() {
    let t = flow_times[i];

    // 1. 市場データの取得 (Index Value & Discount Factor)
    let fwd_rate = model.get_forward_rate(index_pointers[i], t);
    let df = model.get_discount_factor(discount_curve_ids[i], t);

    // 2. フロー計算 (Fused Multiply-Add)
    let float_flow = (fwd_rate * gearing[i]) + spreads[i];
    let total_flow = fixed_amounts[i] + float_flow;

    // 3. 現在価値への集計
    pv += total_flow * df;
}
```

**B. SIMD（ベクトル化）の最大化**
データが連続したメモリ領域（`Vec<f64>`）に配置されているため、コンパイラ（LLVM）は容易に AVX-512 などのSIMD命令を生成できます。一度のCPUサイクルで8個や16個のキャッシュフローを同時に計算可能になります。

**C. 自動微分（Enzyme / AAD）との親和性**
`pricer_models` で課題となっていた AAD の複雑さは、制御フロー（if/match）に起因します。このリニアな計算グラフは、Enzyme が最も効率的に微分コードを生成できる形式です。ジェネリクス汚染も最小限に抑えられます。

**D. メモリ局所性 (Cache Locality)**
階層構造（ポインタ参照）を辿る必要がないため、キャッシュミスが激減します。大規模ポートフォリオ（数万件のトレード）を一括評価する際、この設計は桁違いのスループットを発揮します。

### 3. バリア・オプションなど「経路依存型」への拡張

上記は「線形商品（Linear Products）」向けですが、バリアオプションのような「経路依存型」も、この設計思想を拡張して対応します。

```rust
pub struct ScriptEngine {
    // 観測日
    pub observation_times: Vec<f64>,

    // 状態更新ロジック (イベントIDの列挙)
    // 例: 1=CheckBarrier, 2=Accumulate, 3=Pay
    pub ops_code: Vec<u8>,

    // オペランド (バリア値など)
    pub constants: Vec<f64>,
}
```

これも「イベント駆動」としてフラット化します。複雑なエキゾチック商品も、「イベントの配列」として定義し直すことで、エンジン自体はシンプルなまま保つことができます。

### 実装へのロードマップ

1. **IR定義**: `pricer_core` または `pricer_pricing` に `LinearCashflowStream` 構造体を作成する。
2. **Compiler実装**: `infra_master` の `Trade` を入力とし、`LinearCashflowStream` を出力する `Compiler` トレイトを実装する。ここでカレンダー計算や `Schedule` の展開を全て終わらせる。
3. **Engine差し替え**: 既存の `Pricer` トレイトの実装を、この IR をイテレートするだけの単純なループに置き換える。

実務では「期間（Term）」ではなく「絶対日付（Date）」が契約の正であり、CCY Basis（通貨ベーシス）やCMS（コンベクシティ調整）、Callable（権利行使）といった「非線形・多次元」な要素こそがシステムの複雑性を生む主因です。

これらをシンプルかつ強力に扱うための、もう一段階進化したアーキテクチャとして**「イベント駆動型ベクトルマシン（Event-Driven Vector Machine）」**モデルを提案します。

これは、金融商品を「キャッシュフロー」だけでなく、「状態遷移を引き起こすイベントの連続」として捉え、GPUやSIMD命令セットに最適化した構造です。

---

### 1. 日付と時間の分離（Date-Time Separation）

実務的な日付処理と計算効率を両立させるため、**「契約定義（Date）」と「計算実行（Time）」を明確に分離**します。

* **静的データ（Compile Time）**: 契約上の「日付（例: 2026-03-15）」と、期間係数（YearFraction）は不変です。これらはビルド時（IR生成時）に確定させます。
* **動的データ（Run Time）**: 評価基準日（Valuation Date）からの相対時間のみを動的に計算します。

#### データ構造案（SoA: Structure of Arrays）

```rust
pub struct PricingKernel {
    // --- 1. 日付管理 (i32: Days from Epoch) ---
    // これにより、休日判定や実日数の計算が可能になる
    pub payment_dates: Vec<i32>,     // 支払日
    pub fixing_dates: Vec<i32>,      // 観測日

    // --- 2. 静的計算係数 (f64) ---
    // DayCountConvention (Act/360等) に基づく期間は事前に計算しておく
    pub year_fractions: Vec<f64>,    // τ (tau)

    // --- 3. キャッシュフロー定義 ---
    pub notionals: Vec<f64>,         // 想定元本
    pub spreads: Vec<f64>,           // 固定スプレッド

    // --- 4. 多通貨・多重定義対応 (ID pointers) ---
    pub currency_ids: Vec<u8>,       // 通貨ID (0=USD, 1=JPY, ...)
    pub discount_curve_ids: Vec<u8>, // 割引カーブID (OIS, Collateral curve)
    pub fwd_index_ids: Vec<u16>,     // 参照インデックスID (SOFR, CMS10Y, etc.)
}
```

---

### 2. 複雑な商品の行列化戦略

単純な「金利×期間」に収まらない商品（CCY, CMS, Callable）をどう行列化するか、具体的な解決策を示します。

#### A. CCY Basis (X-Ccy Swap) の行列化： 「通貨次元の追加」

通貨ベーシススワップは、「異なる割引カーブ」と「FX変換」が絡むだけです。これを分岐（if文）で処理せず、**すべてのフローに「FX適用フラグ」と「カーブID」を持たせる**ことでフラット化します。

* **戦略**:
  * 単一通貨スワップでも `currency_id` と `fx_index_id` を持ちます（自国通貨なら FX=1.0 のダミーモデルを指す）。
* **計算式（全商品共通）**: PV = flow * DF * FX
* これにより、USD/JPYベーシススワップも、通常のIRSと同じループで処理可能になります。

#### B. CMS (Constant Maturity Swap) の行列化： 「観測タイプの抽象化」

CMSは「スワップレート（＝正規分布やLogNormalではない分布を持つ）」を観測します。これを扱うには、**`MarketModel` への問い合わせ（Query）をベクトル化**します。

* **戦略**:
  * `fwd_index_ids` が指し示す先を、単純な「LIBOR/SOFRカーブ」だけでなく、「CMS凸性調整済みモデル」も指せるようにします。
  * プライシングエンジン側は「インデックスIDを渡してレートをもらう」という動作を変えません。
* **裏側の仕組み**:
  * `IndexID=5` (USD-SOFR-3M) → 単純なForward Curve lookup
  * `IndexID=20` (USD-CMS-10Y) → Swaption Vol Cube を参照し、凸性調整（Convexity Adjustment）を加えたレートを返す関数
* これにより、エンジンコードを汚さずにCMSをサポートできます。

#### C. Callable Swap / Bermudan の行列化： 「Backward Pass（後退計算）」

ここが最大の難所です。コール条項付きスワップは「未来の価値（Hold Value）」と「行使価値（Exercise Value）」の比較が必要です。これは一本道のストリーム処理では不可能です。

**解決策：LSMC（Longstaff-Schwartz）対応の「ブロック実行モデル」**

IR（中間表現）を**「行使日（Call Date）」で区切られたブロック**として管理します。

1. **Block Structure**:
```rust
struct CallableBlock {
    start_date: i32,
    end_date: i32,
    core_flows: PricingKernel, // この期間内の確定・変動フロー（上記IR）
    exercise_opportunity: Option<ExerciseDef>, // このブロック末尾での行使条件
}
```

2. **実行フロー**:
* **Step 1 (Forward Pass)**: 全ブロックのキャッシュフローを現在価値へ、または各行使時点へ向けて計算・蓄積します。
* **Step 2 (Backward Pass)**: 最終ブロックから過去へ遡り、`exercise_opportunity` がある地点で回帰分析（Regression）を行い、継続価値と行使価値を比較してパスを更新します。

---

### 3. 実装イメージ： 命令セット（Instruction Set）アプローチ

最終的に、Neutryxのプライシングエンジンは、金融商品を**「仮想マシンへの命令列」**として実行する形が最も強力です。

```rust
// エンジンが実行する「命令」の列挙
enum PricingOp {
    // 基本フロー
    CalcFixed { date: i32, amount: f64, ccy: u8 },
    CalcFloat { date: i32, index: u16, gearing: f64, spread: f64, ccy: u8 },

    // 複雑な操作
    ApplyFX { date: i32, target_ccy: u8 }, // 通貨変換
    AccumulatePV,                          // 現在価値バッファに加算

    // 経路依存・条件分岐
    CheckBarrier { obs_date: i32, barrier_level: f64, type: BarrierType },
    CheckExercise { exercise_date: i32, fee: f64 }, // Callable判定
}

// 実行エンジン（イメージ）
fn execute(ops: &[PricingOp], market: &MarketData) -> f64 {
    let mut pv = 0.0;
    let mut current_state = State::new();

    for op in ops {
        match op {
            PricingOp::CalcFloat { date, index, .. } => {
                // CMSだろうがIBORだろうが、market.get_rate(index) で解決
                let rate = market.get_rate(index, date);
                // ...計算とPV加算
            },
            PricingOp::CheckExercise { .. } => {
                // Backward Induction 用の保存処理など
            }
            // ...
        }
    }
    pv
}
```

### 結論：提案するアーキテクチャの要点

1. **Date First**: IRは「絶対日付」を持つ。`YearFraction`はコンパイル時に焼き込む。
2. **Currency as Dimension**: X-Ccy Basis対応のため、通貨とFX変換を全フローの属性として持たせる。
3. **Smart Indices**: CMS対応のため、インデックスIDの参照先（Market Adapter）に凸性調整ロジックを隠蔽する。
4. **Block-Based Execution**: Callable対応のため、商品を「行使日」で分割したブロック配列として管理し、Forward/Backwardの両パスに対応させる。

## Requirements
<!-- Will be generated in /kiro:spec-requirements phase -->
