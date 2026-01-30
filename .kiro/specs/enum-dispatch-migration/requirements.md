# Requirements Document

## Project Description (Input)
2. enum_dispatch (Enum のボイラープレート削減)
金融ライブラリでは、「Instrument Enum が Swap や Bond をラップし、それぞれの price() メソッドを呼び出す」というパターンが頻出します。通常、これには巨大な match 文が必要ですが、enum_dispatch はこれを消滅させます。

画期的な点:

トレイトの実装を Enum の各バリアントに自動転送します。

実行時コストはゼロ（動的ディスパッチ Box<dyn Trait> ではなく、静的な展開が行われるため高速）。

削減イメージ:

```rust
use enum_dispatch::enum_dispatch;

#[enum_dispatch]
trait Pricer {
    fn price(&self) -> f64;
}

// Enum定義に属性をつけるだけ
#[enum_dispatch(Pricer)]
enum Instrument {
    Swap(SwapTrade),
    Bond(BondTrade),
    Option(OptionTrade),
}

// これが不要になる ↓
// impl Pricer for Instrument {
//     fn price(&self) -> f64 {
//         match self {
//             Instrument::Swap(t) => t.price(),
//             Instrument::Bond(t) => t.price(),
//             ...
//         }
//     }
// }
```

## Requirements
<!-- Will be generated in /kiro:spec-requirements phase -->
