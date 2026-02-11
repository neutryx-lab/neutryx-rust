//! Trade direction definitions.

/// Generic trade direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TradeDirection {
    /// Long position (buying).
    Long,
    /// Short position (selling).
    Short,
}

/// Swap trade direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SwapDirection {
    /// Pay fixed rate, receive floating rate.
    PayFixed,
    /// Receive fixed rate, pay floating rate.
    ReceiveFixed,
}

impl From<SwapDirection> for TradeDirection {
    /// Converts SwapDirection to TradeDirection.
    fn from(swap: SwapDirection) -> Self {
        match swap {
            SwapDirection::PayFixed => TradeDirection::Short,
            SwapDirection::ReceiveFixed => TradeDirection::Long,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trade_direction_eq() {
        assert_eq!(TradeDirection::Long, TradeDirection::Long);
        assert_ne!(TradeDirection::Long, TradeDirection::Short);
    }

    #[test]
    fn test_swap_direction_eq() {
        assert_eq!(SwapDirection::PayFixed, SwapDirection::PayFixed);
        assert_ne!(SwapDirection::PayFixed, SwapDirection::ReceiveFixed);
    }

    #[test]
    fn test_swap_to_trade_direction() {
        let pay_fixed: TradeDirection = SwapDirection::PayFixed.into();
        assert_eq!(pay_fixed, TradeDirection::Short);

        let receive_fixed: TradeDirection = SwapDirection::ReceiveFixed.into();
        assert_eq!(receive_fixed, TradeDirection::Long);
    }

    #[test]
    fn test_hash() {
        use std::collections::HashSet;

        let mut trade_set = HashSet::new();
        trade_set.insert(TradeDirection::Long);
        trade_set.insert(TradeDirection::Short);
        trade_set.insert(TradeDirection::Long);
        assert_eq!(trade_set.len(), 2);

        let mut swap_set = HashSet::new();
        swap_set.insert(SwapDirection::PayFixed);
        swap_set.insert(SwapDirection::ReceiveFixed);
        swap_set.insert(SwapDirection::PayFixed);
        assert_eq!(swap_set.len(), 2);
    }

    #[test]
    fn test_clone_copy() {
        let dir1 = TradeDirection::Long;
        let dir2 = dir1;
        let dir3 = dir1.clone();
        assert_eq!(dir1, dir2);
        assert_eq!(dir1, dir3);

        let swap1 = SwapDirection::PayFixed;
        let swap2 = swap1;
        let swap3 = swap1.clone();
        assert_eq!(swap1, swap2);
        assert_eq!(swap1, swap3);
    }

    #[test]
    fn test_debug() {
        let debug = format!("{:?}", TradeDirection::Long);
        assert!(debug.contains("Long"));

        let debug = format!("{:?}", SwapDirection::PayFixed);
        assert!(debug.contains("PayFixed"));
    }
}
