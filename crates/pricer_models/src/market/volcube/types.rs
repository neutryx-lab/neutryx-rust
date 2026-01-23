//! VolCube基本型定義。
//!
//! # Requirements: 1.5, 4.1, 5.1
//!
//! このモジュールはVolCubeで使用する基本的なデータ型を定義する。

use std::time::Instant;

use num_traits::Float;
use serde::{Deserialize, Serialize};

/// Instrument識別子。
///
/// VolCubeのソースInstrumentを一意に識別する。
/// カリブレーション元データの追跡とキャッシュキー生成に使用。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InstrumentId(String);

impl InstrumentId {
    /// 新しいInstrumentIdを作成。
    pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }

    /// 内部文字列への参照を取得。
    pub fn as_str(&self) -> &str { &self.0 }
}

impl std::fmt::Display for InstrumentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
}

impl From<String> for InstrumentId {
    fn from(s: String) -> Self { Self::new(s) }
}

impl From<&str> for InstrumentId {
    fn from(s: &str) -> Self { Self::new(s) }
}

/// Volatility Instrument（Vol Cube入力データ）。
///
/// # Requirements: 1.1, 1.5
///
/// カリブレーション用のボラティリティ観測点を表す。
/// 各点は特定の(expiry, tenor, strike)での市場observed implied vol。
#[derive(Debug, Clone, PartialEq)]
pub struct VolInstrument<T: Float> {
    /// Instrument識別子。
    pub instrument_id: InstrumentId,
    /// Expiry（年単位）。
    pub expiry: T,
    /// Tenor（年単位、SwaptionなどでのUnderlying tenor）。
    pub tenor: T,
    /// Strike（レートまたは価格）。
    pub strike: T,
    /// 市場implied volatility。
    pub implied_vol: T,
    /// Forward rate/price。
    pub forward: T,
    /// カリブレーション重み（デフォルト: 1.0）。
    pub weight: T,
    /// データタイムスタンプ。
    pub timestamp: Instant,
}

impl<T: Float> VolInstrument<T> {
    /// 新しいVolInstrumentを作成。
    pub fn new(
        instrument_id: impl Into<InstrumentId>,
        expiry: T,
        tenor: T,
        strike: T,
        implied_vol: T,
        forward: T,
    ) -> Self {
        Self {
            instrument_id: instrument_id.into(),
            expiry,
            tenor,
            strike,
            implied_vol,
            forward,
            weight: T::one(),
            timestamp: Instant::now(),
        }
    }

    /// 重みを設定。
    pub fn with_weight(mut self, weight: T) -> Self {
        self.weight = weight;
        self
    }

    /// タイムスタンプを設定。
    pub fn with_timestamp(mut self, timestamp: Instant) -> Self {
        self.timestamp = timestamp;
        self
    }

    /// Instrumentを検証。
    pub fn validate(&self) -> Result<(), String> {
        let zero = T::zero();
        if self.expiry <= zero {
            return Err("Expiry must be positive".to_string());
        }
        if self.tenor <= zero {
            return Err("Tenor must be positive".to_string());
        }
        if self.strike <= zero {
            return Err("Strike must be positive".to_string());
        }
        if self.implied_vol <= zero {
            return Err("Implied volatility must be positive".to_string());
        }
        if self.forward <= zero {
            return Err("Forward must be positive".to_string());
        }
        if self.weight <= zero {
            return Err("Weight must be positive".to_string());
        }
        Ok(())
    }
}

/// SABR parameters for a single expiry-tenor slice.
///
/// # Requirements: 1.2
///
/// Hagan公式で使用するSABRパラメータセット。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SabrParams<T: Float> {
    /// Alpha: ATM volatility level。
    pub alpha: T,
    /// Beta: CEV exponent (typically fixed)。
    pub beta: T,
    /// Rho: spot-vol correlation (-1 < rho < 1)。
    pub rho: T,
    /// Nu: vol-of-vol (volvol)。
    pub nu: T,
}

impl<T: Float> SabrParams<T> {
    /// 新しいSabrParamsを作成。
    pub fn new(alpha: T, beta: T, rho: T, nu: T) -> Self {
        Self {
            alpha,
            beta,
            rho,
            nu,
        }
    }

    /// パラメータを検証。
    pub fn validate(&self) -> Result<(), String> {
        let zero = T::zero();
        let one = T::one();
        let neg_one = -one;

        if self.alpha <= zero {
            return Err("Alpha must be positive".to_string());
        }
        if self.beta < zero || self.beta > one {
            return Err("Beta must be in [0, 1]".to_string());
        }
        if self.rho <= neg_one || self.rho >= one {
            return Err("Rho must be in (-1, 1)".to_string());
        }
        if self.nu <= zero {
            return Err("Nu must be positive".to_string());
        }
        Ok(())
    }
}

impl<T: Float> Default for SabrParams<T> {
    fn default() -> Self {
        Self {
            alpha: T::from(0.04).unwrap(),
            beta: T::from(0.5).unwrap(),
            rho: T::from(-0.3).unwrap(),
            nu: T::from(0.4).unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // InstrumentId Tests
    // =========================================================================

    #[test]
    fn test_instrument_id_new() {
        let id = InstrumentId::new("EUR-USD-1Y5Y-ATM");
        assert_eq!(id.as_str(), "EUR-USD-1Y5Y-ATM");
    }

    #[test]
    fn test_instrument_id_from_string() {
        let id: InstrumentId = "SWAPTION-123".to_string().into();
        assert_eq!(id.as_str(), "SWAPTION-123");
    }

    #[test]
    fn test_instrument_id_from_str() {
        let id: InstrumentId = "CAPLET-456".into();
        assert_eq!(id.as_str(), "CAPLET-456");
    }

    #[test]
    fn test_instrument_id_display() {
        let id = InstrumentId::new("TEST-ID");
        assert_eq!(format!("{}", id), "TEST-ID");
    }

    #[test]
    fn test_instrument_id_clone_eq_hash() {
        use std::collections::HashSet;
        let id1 = InstrumentId::new("ID-1");
        let id2 = id1.clone();
        assert_eq!(id1, id2);

        let mut set = HashSet::new();
        set.insert(id1.clone());
        assert!(set.contains(&id1));
    }

    // =========================================================================
    // VolInstrument Tests
    // =========================================================================

    #[test]
    fn test_vol_instrument_new() {
        let inst = VolInstrument::new(
            "TEST-1", 1.0_f64, // expiry
            5.0,     // tenor
            0.03,    // strike
            0.20,    // implied_vol
            0.03,    // forward
        );
        assert_eq!(inst.instrument_id.as_str(), "TEST-1");
        assert_eq!(inst.expiry, 1.0);
        assert_eq!(inst.tenor, 5.0);
        assert_eq!(inst.strike, 0.03);
        assert_eq!(inst.implied_vol, 0.20);
        assert_eq!(inst.forward, 0.03);
        assert_eq!(inst.weight, 1.0);
    }

    #[test]
    fn test_vol_instrument_with_weight() {
        let inst = VolInstrument::new("TEST", 1.0_f64, 5.0, 0.03, 0.20, 0.03).with_weight(2.0);
        assert_eq!(inst.weight, 2.0);
    }

    #[test]
    fn test_vol_instrument_validate_valid() {
        let inst = VolInstrument::new("TEST", 1.0_f64, 5.0, 0.03, 0.20, 0.03);
        assert!(inst.validate().is_ok());
    }

    #[test]
    fn test_vol_instrument_validate_negative_expiry() {
        let inst = VolInstrument::new("TEST", -1.0_f64, 5.0, 0.03, 0.20, 0.03);
        let result = inst.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Expiry"));
    }

    #[test]
    fn test_vol_instrument_validate_zero_strike() {
        let inst = VolInstrument::new("TEST", 1.0_f64, 5.0, 0.0, 0.20, 0.03);
        let result = inst.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Strike"));
    }

    #[test]
    fn test_vol_instrument_validate_negative_vol() {
        let inst = VolInstrument::new("TEST", 1.0_f64, 5.0, 0.03, -0.20, 0.03);
        let result = inst.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("volatility"));
    }

    // =========================================================================
    // SabrParams Tests
    // =========================================================================

    #[test]
    fn test_sabr_params_new() {
        let params = SabrParams::new(0.04_f64, 0.5, -0.3, 0.4);
        assert_eq!(params.alpha, 0.04);
        assert_eq!(params.beta, 0.5);
        assert_eq!(params.rho, -0.3);
        assert_eq!(params.nu, 0.4);
    }

    #[test]
    fn test_sabr_params_default() {
        let params = SabrParams::<f64>::default();
        assert_eq!(params.alpha, 0.04);
        assert_eq!(params.beta, 0.5);
        assert_eq!(params.rho, -0.3);
        assert_eq!(params.nu, 0.4);
    }

    #[test]
    fn test_sabr_params_validate_valid() {
        let params = SabrParams::new(0.04_f64, 0.5, -0.3, 0.4);
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_sabr_params_validate_alpha_negative() {
        let params = SabrParams::new(-0.04_f64, 0.5, -0.3, 0.4);
        let result = params.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Alpha"));
    }

    #[test]
    fn test_sabr_params_validate_beta_out_of_range() {
        let params = SabrParams::new(0.04_f64, 1.5, -0.3, 0.4);
        let result = params.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Beta"));
    }

    #[test]
    fn test_sabr_params_validate_rho_out_of_range() {
        let params = SabrParams::new(0.04_f64, 0.5, 1.0, 0.4);
        let result = params.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Rho"));
    }

    #[test]
    fn test_sabr_params_validate_nu_negative() {
        let params = SabrParams::new(0.04_f64, 0.5, -0.3, -0.4);
        let result = params.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Nu"));
    }

    #[test]
    fn test_sabr_params_copy() {
        let params1 = SabrParams::new(0.04_f64, 0.5, -0.3, 0.4);
        let params2 = params1;
        assert_eq!(params1, params2);
    }
}
