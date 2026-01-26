# Requirements Document

## Project Description (Input)
前回の「巨大な配列ですべてを管理する（Arena）」案は、確かにメモリ効率は最強ですが、実装と保守のコストが高すぎました。実務的な開発速度を損なっては本末転倒です。

ご要望の**「マーケットを配列に再配置する手間をなくす」**かつ**「ジェネリクス汚染を回避する」**ための、最もシンプルで強力なアプローチを提案します。

それは、**「シャドウ・オブジェクト（Shadow Object）」パターン**です。

---

### コンセプト：データ構造は「リッチ」に、カーネルは「プリミティブ」に

「マーケット全体を1つの配列にする」必要はありません。既存の `Vec<f64>`（例えば `YieldCurve` 内のレート配列）を**そのまま**使い、その参照（スライス）だけを計算カーネルに渡します。

Enzymeの強力な点は、**「ポインタ（参照）の先にあるデータ」も微分できる**ことです。

#### 新アーキテクチャの3つのルール

1. **データ構造は変えない**: `struct Market`, `struct YieldCurve` は今のままでOK。ジェネリクス `T` も不要です。
2. **Shadow（勾配）は `clone` で作る**: 計算開始時に、Market構造体と同じ形をした「勾配用構造体（全てゼロ）」を `clone` で作ります。
3. **カーネルは `&[f64]` を取る**: プライシング関数は、構造体そのものではなく、そこから取り出した `&[f64]`（スライス）を引数に取ります。

---

### 具体的な実装（Rust）

これなら、面倒なID管理やデータの並べ替えは一切不要です。変数名がそのままマッピングになります。

#### 1. データ定義（既存のままでOK）

一切のジェネリクスを排除し、純粋な `f64` で定義します。

```rust
#[derive(Clone)] // Cloneの実装は必須（Shadow作成用）
pub struct YieldCurve {
    pub rates: Vec<f64>, // これをそのまま微分対象にする
    pub times: Vec<f64>,
}

pub struct Swap {
    pub notionals: Vec<f64>,
    pub year_fractions: Vec<f64>,
}

```

#### 2. 計算カーネル（ここがポイント）

構造体を受け取るメソッドではなく、**数値スライスを受け取る関数**として定義します。これが「AAD境界」になります。

```rust
#[no_mangle] // Enzymeが見つけられるように
fn pricing_kernel(
    // --- Active Inputs (微分対象) ---
    rates: &[f64],
    // --- Constant Inputs (定数) ---
    times: &[f64],
    notionals: &[f64],
    year_fractions: &[f64],
    // --- Output ---
    output: &mut f64
) {
    let n = rates.len();
    let mut pv = 0.0;

    // ここは普通のf64計算。Dualなどは一切不要。
    for i in 0..n {
        let df = (-rates[i] * times[i]).exp();
        pv += notionals[i] * (rates[i] - 0.03) * year_fractions[i] * df;
    }

    *output = pv;
}

```

#### 3. AAD実行（バインダー層）

ここが「魔法」の部分です。データの詰め替え（シリアライズ）は行いません。**ポインタ（参照）を渡すだけ**です。

```rust
pub fn calculate_risk(market: &YieldCurve, trade: &Swap) -> (f64, YieldCurve) {
    // 1. Shadow Objectの作成（ここが唯一のオーバーヘッドだが、アロケーションは一瞬）
    // marketと同じ構造で、値がすべて0.0の構造体を作る
    let mut d_market = market.clone();
    fill_zeros(&mut d_market); // 全て0.0にするヘルパー関数を用意しておく

    let mut pv = 0.0;
    let mut d_pv = 1.0; // Seed Gradient (PVに対する感度 = 1.0)

    unsafe {
        __enzyme_autodiff(
            pricing_kernel,
            // Rates: Duplicated (値の参照 と 勾配の参照 を渡す)
            enzyme_consts::ENZYME_DUP,
            market.rates.as_ptr(), d_market.rates.as_mut_ptr(), market.rates.len(),

            // Times: Const (値の参照のみ)
            enzyme_consts::ENZYME_CONST,
            market.times.as_ptr(), market.times.len(),

            // Notionals: Const
            enzyme_consts::ENZYME_CONST,
            trade.notionals.as_ptr(), trade.notionals.len(),

            // YearFractions: Const
            enzyme_consts::ENZYME_CONST,
            trade.year_fractions.as_ptr(), trade.year_fractions.len(),

            // Output
            enzyme_consts::ENZYME_DUP,
            &mut pv, &mut d_pv,
        );
    }

    // この時点で、d_market.rates に各レートに対する感度（Delta）が入っている！
    // IDマッピング等の後処理は不要。d_market がそのまま結果の構造体になる。

    (pv, d_market)
}

```

---

### なぜこれが「強力」なのか？

1. **「構造」と「値」の完全一致**:
* `d_market` は `market` と全く同じ型 (`YieldCurve`) です。
* 「3番目のレートのリスクはどこ？」→ `d_market.rates[2]` にあります。直感的で、デバッグが極めて容易です。


2. **ゼロ・コピー**:
* 計算時に巨大配列へのコピー（Pack/Unpack）は発生しません。既存の `Vec` のポインタを渡しているだけです。


3. **部分適用の柔軟性**:
* もしVolSurfaceも微分したければ、カーネルの引数に `vols: &[f64]` を追加し、呼び出し側で `market.vols` を渡すだけです。


4. **コンパイル時間の短縮**:
* `pricing_kernel` は `f64` 専用の関数1つだけです。ジェネリクスの展開（Monomorphization）による爆発が起きません。



### 実務的な導入手順

このアプローチを採用する場合、以下の手順で進めるのが最もスムーズです。

1. **`Shadow` トレイトの導入**:
```rust
trait Shadow: Clone {
    fn zero_out(&mut self); // 全フィールドを0.0にする
}
// これをMarketデータ構造体にderiveまたはimplする

```


2. **カーネル関数の分離**:
既存の `impl Swap { fn price(...) }` の中身を、`fn swap_pricing_kernel(...)` という自由関数（Free Function）に移動させます。引数はすべてスライス `&[f64]` にします。
3. **FFIバインディング**:
`extern "C"` ブロックで Enzyme の呼び出しを一箇所に記述します。

この方法なら、既存のNeutryxのデータモデルを破壊せず、**「計算ロジックの一部だけをsurgicalに（外科手術的に）AAD化」**できます。「配列管理の面倒さ」への完璧な回答になるはずです。

## Requirements
<!-- Will be generated in /kiro:spec-requirements phase -->
