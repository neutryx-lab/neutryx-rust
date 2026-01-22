//! Sample market data generation.
//!
//! This module provides realistic sample market data for the Market Data Viewer.
//!
//! # Task Coverage
//!
//! - Task 2.1: USD interest rate data (Deposit, Swap, OIS)
//! - Task 2.2: EUR interest rate data
//! - Task 2.3: JPY interest rate data
//! - Task 2.4: FX spot rates
//! - Task 2.5: Convention presets
//!
//! # Data Generation
//!
//! The module generates realistic market data with:
//! - Standard tenor points for curve construction
//! - Realistic rate levels based on market conditions
//! - Proper bid/ask/mid quoting
//! - Data source attribution

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

use super::market_types::{
    ConventionField, ConventionResponse, ConventionSummary, ConventionsListResponse,
    InstrumentResponse, MarketRateDetailResponse, MarketRateQuery, MarketRateResponse,
    MarketRatesListResponse,
};

// =============================================================================
// Market Data Cache
// =============================================================================

/// Thread-safe market data cache.
#[derive(Debug)]
pub struct MarketDataCache {
    /// Cached rates (thread-safe).
    rates: tokio::sync::RwLock<Vec<MarketRateResponse>>,
    /// Last update timestamp.
    last_updated: AtomicI64,
    /// Staleness threshold in milliseconds (5 minutes).
    staleness_threshold_ms: i64,
}

impl Default for MarketDataCache {
    fn default() -> Self {
        Self::new()
    }
}

impl MarketDataCache {
    /// Creates a new cache with sample data.
    pub fn new() -> Self {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let rates = generate_all_sample_rates(now_ms);

        Self {
            rates: tokio::sync::RwLock::new(rates),
            last_updated: AtomicI64::new(now_ms),
            staleness_threshold_ms: 5 * 60 * 1000, // 5 minutes
        }
    }

    /// Gets the current market rates, optionally filtered.
    pub async fn get_rates(&self, query: &MarketRateQuery) -> MarketRatesListResponse {
        let rates = self.rates.read().await;
        let now_ms = chrono::Utc::now().timestamp_millis();
        let threshold = now_ms - self.staleness_threshold_ms;

        let filtered: Vec<MarketRateResponse> = rates
            .iter()
            .filter(|r| {
                // Apply currency filter
                if let Some(ref currency) = query.currency {
                    if !r.currency.eq_ignore_ascii_case(currency) {
                        return false;
                    }
                }
                // Apply rate_type filter
                if let Some(ref rate_type) = query.rate_type {
                    if !r.rate_type.eq_ignore_ascii_case(rate_type) {
                        return false;
                    }
                }
                // Apply index filter
                if let Some(ref index) = query.index {
                    match &r.rate_index {
                        Some(ri) => {
                            if !ri.eq_ignore_ascii_case(index) {
                                return false;
                            }
                        }
                        None => return false,
                    }
                }
                true
            })
            .map(|r| MarketRateResponse {
                is_stale: r.timestamp < threshold,
                ..r.clone()
            })
            .collect();

        let total_count = filtered.len();

        MarketRatesListResponse {
            rates: filtered,
            last_updated: self.last_updated.load(Ordering::Relaxed),
            total_count,
        }
    }

    /// Gets a single rate by ID.
    pub async fn get_rate(&self, rate_id: &str) -> Option<MarketRateResponse> {
        let rates = self.rates.read().await;
        let now_ms = chrono::Utc::now().timestamp_millis();
        let threshold = now_ms - self.staleness_threshold_ms;

        rates.iter().find(|r| r.id == rate_id).map(|r| {
            MarketRateResponse {
                is_stale: r.timestamp < threshold,
                ..r.clone()
            }
        })
    }

    /// Gets detailed information for a rate including instrument and convention.
    pub async fn get_rate_detail(&self, rate_id: &str) -> Option<MarketRateDetailResponse> {
        let rate = self.get_rate(rate_id).await?;

        // Generate instrument information based on rate type
        let instrument = generate_instrument_for_rate(&rate);

        // Generate convention information based on currency and rate type
        let convention = generate_convention_for_rate(&rate);

        Some(MarketRateDetailResponse {
            rate,
            instrument,
            convention,
        })
    }

    /// Refreshes rates with small random perturbations.
    pub async fn refresh(&self) {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let mut rates = self.rates.write().await;

        // Apply small random perturbations to simulate market movement
        for rate in rates.iter_mut() {
            // Small perturbation: +/- 0.5 bps
            let perturbation = (pseudo_random(rate.timestamp) % 10) as f64 / 10000.0 - 0.0005;
            rate.value += perturbation;
            rate.timestamp = now_ms;
        }

        self.last_updated.store(now_ms, Ordering::Relaxed);
    }

    /// Gets the last update timestamp.
    pub fn last_updated(&self) -> i64 {
        self.last_updated.load(Ordering::Relaxed)
    }
}

// =============================================================================
// Sample Data Generation - Task 2.1-2.4
// =============================================================================

/// Generates all sample market rates.
fn generate_all_sample_rates(timestamp: i64) -> Vec<MarketRateResponse> {
    let mut rates = Vec::new();

    // Task 2.1: USD rates
    rates.extend(generate_usd_rates(timestamp));

    // Task 2.2: EUR rates
    rates.extend(generate_eur_rates(timestamp));

    // Task 2.3: JPY rates
    rates.extend(generate_jpy_rates(timestamp));

    // Task 2.4: FX spot rates
    rates.extend(generate_fx_rates(timestamp));

    rates
}

/// Task 2.1: Generates USD interest rate data.
fn generate_usd_rates(timestamp: i64) -> Vec<MarketRateResponse> {
    let mut rates = Vec::new();

    // USD Deposit rates
    let deposit_tenors = [("ON", 0.0525), ("1W", 0.0528), ("1M", 0.0532), ("3M", 0.0538)];

    for (tenor, value) in deposit_tenors {
        rates.push(MarketRateResponse {
            id: format!("USD-{}-DEPO", tenor),
            currency: "USD".to_string(),
            tenor: tenor.to_string(),
            rate_type: "Deposit".to_string(),
            value,
            quote_type: "Mid".to_string(),
            timestamp,
            source: "Bloomberg".to_string(),
            is_stale: false,
            rate_index: None,
        });
    }

    // USD SOFR OIS rates
    let ois_tenors = [
        ("1M", 0.0528),
        ("3M", 0.0530),
        ("6M", 0.0515),
        ("1Y", 0.0475),
        ("2Y", 0.0420),
        ("3Y", 0.0395),
        ("5Y", 0.0380),
        ("7Y", 0.0378),
        ("10Y", 0.0382),
        ("15Y", 0.0388),
        ("20Y", 0.0392),
        ("30Y", 0.0395),
    ];

    for (tenor, value) in ois_tenors {
        rates.push(MarketRateResponse {
            id: format!("USD-{}-OIS", tenor),
            currency: "USD".to_string(),
            tenor: tenor.to_string(),
            rate_type: "Ois".to_string(),
            value,
            quote_type: "Mid".to_string(),
            timestamp,
            source: "Bloomberg".to_string(),
            is_stale: false,
            rate_index: Some("SOFR".to_string()),
        });
    }

    // USD LIBOR/Term SOFR Swap rates (legacy convention)
    let swap_tenors = [
        ("2Y", 0.0435),
        ("3Y", 0.0415),
        ("5Y", 0.0402),
        ("7Y", 0.0400),
        ("10Y", 0.0408),
        ("30Y", 0.0425),
    ];

    for (tenor, value) in swap_tenors {
        rates.push(MarketRateResponse {
            id: format!("USD-{}-SWAP", tenor),
            currency: "USD".to_string(),
            tenor: tenor.to_string(),
            rate_type: "Swap".to_string(),
            value,
            quote_type: "Mid".to_string(),
            timestamp,
            source: "Bloomberg".to_string(),
            is_stale: false,
            rate_index: Some("SOFR".to_string()),
        });
    }

    rates
}

/// Task 2.2: Generates EUR interest rate data.
fn generate_eur_rates(timestamp: i64) -> Vec<MarketRateResponse> {
    let mut rates = Vec::new();

    // EUR Deposit rates
    let deposit_tenors = [("ON", 0.0390), ("1W", 0.0392), ("1M", 0.0395), ("3M", 0.0398)];

    for (tenor, value) in deposit_tenors {
        rates.push(MarketRateResponse {
            id: format!("EUR-{}-DEPO", tenor),
            currency: "EUR".to_string(),
            tenor: tenor.to_string(),
            rate_type: "Deposit".to_string(),
            value,
            quote_type: "Mid".to_string(),
            timestamp,
            source: "Bloomberg".to_string(),
            is_stale: false,
            rate_index: None,
        });
    }

    // EUR ESTR OIS rates
    let ois_tenors = [
        ("1M", 0.0390),
        ("3M", 0.0388),
        ("6M", 0.0375),
        ("1Y", 0.0345),
        ("2Y", 0.0295),
        ("3Y", 0.0270),
        ("5Y", 0.0255),
        ("10Y", 0.0275),
        ("30Y", 0.0285),
    ];

    for (tenor, value) in ois_tenors {
        rates.push(MarketRateResponse {
            id: format!("EUR-{}-OIS", tenor),
            currency: "EUR".to_string(),
            tenor: tenor.to_string(),
            rate_type: "Ois".to_string(),
            value,
            quote_type: "Mid".to_string(),
            timestamp,
            source: "Bloomberg".to_string(),
            is_stale: false,
            rate_index: Some("ESTR".to_string()),
        });
    }

    // EUR EURIBOR Swap rates
    let swap_tenors = [
        ("2Y", 0.0310),
        ("3Y", 0.0290),
        ("5Y", 0.0275),
        ("10Y", 0.0295),
        ("30Y", 0.0310),
    ];

    for (tenor, value) in swap_tenors {
        rates.push(MarketRateResponse {
            id: format!("EUR-{}-SWAP", tenor),
            currency: "EUR".to_string(),
            tenor: tenor.to_string(),
            rate_type: "Swap".to_string(),
            value,
            quote_type: "Mid".to_string(),
            timestamp,
            source: "Bloomberg".to_string(),
            is_stale: false,
            rate_index: Some("EURIBOR".to_string()),
        });
    }

    rates
}

/// Task 2.3: Generates JPY interest rate data.
fn generate_jpy_rates(timestamp: i64) -> Vec<MarketRateResponse> {
    let mut rates = Vec::new();

    // JPY Deposit rates (very low rates)
    let deposit_tenors = [("ON", -0.0001), ("1W", 0.0000), ("1M", 0.0002), ("3M", 0.0005)];

    for (tenor, value) in deposit_tenors {
        rates.push(MarketRateResponse {
            id: format!("JPY-{}-DEPO", tenor),
            currency: "JPY".to_string(),
            tenor: tenor.to_string(),
            rate_type: "Deposit".to_string(),
            value,
            quote_type: "Mid".to_string(),
            timestamp,
            source: "Bloomberg".to_string(),
            is_stale: false,
            rate_index: None,
        });
    }

    // JPY TONA OIS rates
    let ois_tenors = [
        ("1M", 0.0001),
        ("3M", 0.0003),
        ("6M", 0.0008),
        ("1Y", 0.0020),
        ("2Y", 0.0055),
        ("5Y", 0.0095),
        ("10Y", 0.0145),
        ("30Y", 0.0205),
    ];

    for (tenor, value) in ois_tenors {
        rates.push(MarketRateResponse {
            id: format!("JPY-{}-OIS", tenor),
            currency: "JPY".to_string(),
            tenor: tenor.to_string(),
            rate_type: "Ois".to_string(),
            value,
            quote_type: "Mid".to_string(),
            timestamp,
            source: "Bloomberg".to_string(),
            is_stale: false,
            rate_index: Some("TONA".to_string()),
        });
    }

    // JPY TIBOR Swap rates
    let swap_tenors = [
        ("2Y", 0.0065),
        ("5Y", 0.0110),
        ("10Y", 0.0165),
        ("30Y", 0.0230),
    ];

    for (tenor, value) in swap_tenors {
        rates.push(MarketRateResponse {
            id: format!("JPY-{}-SWAP", tenor),
            currency: "JPY".to_string(),
            tenor: tenor.to_string(),
            rate_type: "Swap".to_string(),
            value,
            quote_type: "Mid".to_string(),
            timestamp,
            source: "Bloomberg".to_string(),
            is_stale: false,
            rate_index: Some("TIBOR".to_string()),
        });
    }

    rates
}

/// Task 2.4: Generates FX spot rates.
fn generate_fx_rates(timestamp: i64) -> Vec<MarketRateResponse> {
    vec![
        MarketRateResponse {
            id: "EURUSD-SPOT".to_string(),
            currency: "EUR".to_string(),
            tenor: "SPOT".to_string(),
            rate_type: "FxSpot".to_string(),
            value: 1.0875,
            quote_type: "Mid".to_string(),
            timestamp,
            source: "Reuters".to_string(),
            is_stale: false,
            rate_index: None,
        },
        MarketRateResponse {
            id: "USDJPY-SPOT".to_string(),
            currency: "USD".to_string(),
            tenor: "SPOT".to_string(),
            rate_type: "FxSpot".to_string(),
            value: 154.25,
            quote_type: "Mid".to_string(),
            timestamp,
            source: "Reuters".to_string(),
            is_stale: false,
            rate_index: None,
        },
        MarketRateResponse {
            id: "GBPUSD-SPOT".to_string(),
            currency: "GBP".to_string(),
            tenor: "SPOT".to_string(),
            rate_type: "FxSpot".to_string(),
            value: 1.2645,
            quote_type: "Mid".to_string(),
            timestamp,
            source: "Reuters".to_string(),
            is_stale: false,
            rate_index: None,
        },
        MarketRateResponse {
            id: "EURGBP-SPOT".to_string(),
            currency: "EUR".to_string(),
            tenor: "SPOT".to_string(),
            rate_type: "FxSpot".to_string(),
            value: 0.8600,
            quote_type: "Mid".to_string(),
            timestamp,
            source: "Reuters".to_string(),
            is_stale: false,
            rate_index: None,
        },
        MarketRateResponse {
            id: "EURJPY-SPOT".to_string(),
            currency: "EUR".to_string(),
            tenor: "SPOT".to_string(),
            rate_type: "FxSpot".to_string(),
            value: 167.75,
            quote_type: "Mid".to_string(),
            timestamp,
            source: "Reuters".to_string(),
            is_stale: false,
            rate_index: None,
        },
    ]
}

// =============================================================================
// Task 2.5: Convention Presets
// =============================================================================

/// Gets all available conventions.
pub fn get_conventions_list() -> ConventionsListResponse {
    ConventionsListResponse {
        conventions: vec![
            // USD Conventions
            ConventionSummary {
                id: "USD-SOFR-OIS".to_string(),
                currency: "USD".to_string(),
                convention_type: "OisConvention".to_string(),
                is_default: true,
            },
            ConventionSummary {
                id: "USD-SOFR-SWAP".to_string(),
                currency: "USD".to_string(),
                convention_type: "SwapConvention".to_string(),
                is_default: true,
            },
            ConventionSummary {
                id: "USD-DEPO".to_string(),
                currency: "USD".to_string(),
                convention_type: "DepositConvention".to_string(),
                is_default: true,
            },
            // EUR Conventions
            ConventionSummary {
                id: "EUR-ESTR-OIS".to_string(),
                currency: "EUR".to_string(),
                convention_type: "OisConvention".to_string(),
                is_default: true,
            },
            ConventionSummary {
                id: "EUR-EURIBOR-SWAP".to_string(),
                currency: "EUR".to_string(),
                convention_type: "SwapConvention".to_string(),
                is_default: true,
            },
            ConventionSummary {
                id: "EUR-DEPO".to_string(),
                currency: "EUR".to_string(),
                convention_type: "DepositConvention".to_string(),
                is_default: true,
            },
            // JPY Conventions
            ConventionSummary {
                id: "JPY-TONA-OIS".to_string(),
                currency: "JPY".to_string(),
                convention_type: "OisConvention".to_string(),
                is_default: true,
            },
            ConventionSummary {
                id: "JPY-TIBOR-SWAP".to_string(),
                currency: "JPY".to_string(),
                convention_type: "SwapConvention".to_string(),
                is_default: true,
            },
            ConventionSummary {
                id: "JPY-DEPO".to_string(),
                currency: "JPY".to_string(),
                convention_type: "DepositConvention".to_string(),
                is_default: true,
            },
            // FX Conventions
            ConventionSummary {
                id: "FX-SPOT".to_string(),
                currency: "USD".to_string(),
                convention_type: "FxConvention".to_string(),
                is_default: true,
            },
        ],
    }
}

/// Gets a convention by ID.
pub fn get_convention(convention_id: &str) -> Option<ConventionResponse> {
    match convention_id {
        "USD-SOFR-OIS" => Some(ConventionResponse {
            convention_type: "OisConvention".to_string(),
            fields: vec![
                ConventionField {
                    label: "Index".to_string(),
                    value: "SOFR".to_string(),
                },
                ConventionField {
                    label: "Day Count".to_string(),
                    value: "ACT/360".to_string(),
                },
                ConventionField {
                    label: "Payment Frequency".to_string(),
                    value: "Annual".to_string(),
                },
                ConventionField {
                    label: "Business Day Convention".to_string(),
                    value: "Modified Following".to_string(),
                },
                ConventionField {
                    label: "Settlement Days".to_string(),
                    value: "2".to_string(),
                },
            ],
        }),
        "USD-SOFR-SWAP" => Some(ConventionResponse {
            convention_type: "SwapConvention".to_string(),
            fields: vec![
                ConventionField {
                    label: "Fixed Leg Index".to_string(),
                    value: "SOFR".to_string(),
                },
                ConventionField {
                    label: "Fixed Leg Day Count".to_string(),
                    value: "ACT/360".to_string(),
                },
                ConventionField {
                    label: "Fixed Leg Frequency".to_string(),
                    value: "Annual".to_string(),
                },
                ConventionField {
                    label: "Float Leg Index".to_string(),
                    value: "SOFR".to_string(),
                },
                ConventionField {
                    label: "Float Leg Day Count".to_string(),
                    value: "ACT/360".to_string(),
                },
                ConventionField {
                    label: "Float Leg Frequency".to_string(),
                    value: "Annual".to_string(),
                },
                ConventionField {
                    label: "Business Day Convention".to_string(),
                    value: "Modified Following".to_string(),
                },
            ],
        }),
        "USD-DEPO" => Some(ConventionResponse {
            convention_type: "DepositConvention".to_string(),
            fields: vec![
                ConventionField {
                    label: "Day Count".to_string(),
                    value: "ACT/360".to_string(),
                },
                ConventionField {
                    label: "Business Day Convention".to_string(),
                    value: "Modified Following".to_string(),
                },
                ConventionField {
                    label: "Settlement Days".to_string(),
                    value: "2".to_string(),
                },
            ],
        }),
        "EUR-ESTR-OIS" => Some(ConventionResponse {
            convention_type: "OisConvention".to_string(),
            fields: vec![
                ConventionField {
                    label: "Index".to_string(),
                    value: "ESTR".to_string(),
                },
                ConventionField {
                    label: "Day Count".to_string(),
                    value: "ACT/360".to_string(),
                },
                ConventionField {
                    label: "Payment Frequency".to_string(),
                    value: "Annual".to_string(),
                },
                ConventionField {
                    label: "Business Day Convention".to_string(),
                    value: "Modified Following".to_string(),
                },
            ],
        }),
        "EUR-EURIBOR-SWAP" => Some(ConventionResponse {
            convention_type: "SwapConvention".to_string(),
            fields: vec![
                ConventionField {
                    label: "Fixed Leg Day Count".to_string(),
                    value: "30/360".to_string(),
                },
                ConventionField {
                    label: "Fixed Leg Frequency".to_string(),
                    value: "Annual".to_string(),
                },
                ConventionField {
                    label: "Float Leg Index".to_string(),
                    value: "EURIBOR 6M".to_string(),
                },
                ConventionField {
                    label: "Float Leg Day Count".to_string(),
                    value: "ACT/360".to_string(),
                },
                ConventionField {
                    label: "Float Leg Frequency".to_string(),
                    value: "Semi-Annual".to_string(),
                },
            ],
        }),
        "EUR-DEPO" => Some(ConventionResponse {
            convention_type: "DepositConvention".to_string(),
            fields: vec![
                ConventionField {
                    label: "Day Count".to_string(),
                    value: "ACT/360".to_string(),
                },
                ConventionField {
                    label: "Business Day Convention".to_string(),
                    value: "Modified Following".to_string(),
                },
                ConventionField {
                    label: "Settlement Days".to_string(),
                    value: "2".to_string(),
                },
            ],
        }),
        "JPY-TONA-OIS" => Some(ConventionResponse {
            convention_type: "OisConvention".to_string(),
            fields: vec![
                ConventionField {
                    label: "Index".to_string(),
                    value: "TONA".to_string(),
                },
                ConventionField {
                    label: "Day Count".to_string(),
                    value: "ACT/365F".to_string(),
                },
                ConventionField {
                    label: "Payment Frequency".to_string(),
                    value: "Annual".to_string(),
                },
                ConventionField {
                    label: "Business Day Convention".to_string(),
                    value: "Modified Following".to_string(),
                },
            ],
        }),
        "JPY-TIBOR-SWAP" => Some(ConventionResponse {
            convention_type: "SwapConvention".to_string(),
            fields: vec![
                ConventionField {
                    label: "Fixed Leg Day Count".to_string(),
                    value: "ACT/365F".to_string(),
                },
                ConventionField {
                    label: "Fixed Leg Frequency".to_string(),
                    value: "Semi-Annual".to_string(),
                },
                ConventionField {
                    label: "Float Leg Index".to_string(),
                    value: "TIBOR 6M".to_string(),
                },
                ConventionField {
                    label: "Float Leg Day Count".to_string(),
                    value: "ACT/365F".to_string(),
                },
                ConventionField {
                    label: "Float Leg Frequency".to_string(),
                    value: "Semi-Annual".to_string(),
                },
            ],
        }),
        "JPY-DEPO" => Some(ConventionResponse {
            convention_type: "DepositConvention".to_string(),
            fields: vec![
                ConventionField {
                    label: "Day Count".to_string(),
                    value: "ACT/365F".to_string(),
                },
                ConventionField {
                    label: "Business Day Convention".to_string(),
                    value: "Modified Following".to_string(),
                },
                ConventionField {
                    label: "Settlement Days".to_string(),
                    value: "2".to_string(),
                },
            ],
        }),
        "FX-SPOT" => Some(ConventionResponse {
            convention_type: "FxConvention".to_string(),
            fields: vec![
                ConventionField {
                    label: "Settlement Days".to_string(),
                    value: "2".to_string(),
                },
                ConventionField {
                    label: "Business Day Convention".to_string(),
                    value: "Following".to_string(),
                },
            ],
        }),
        _ => None,
    }
}

// =============================================================================
// Instrument and Convention Generation for Rate Detail
// =============================================================================

/// Generates instrument information for a given rate.
fn generate_instrument_for_rate(rate: &MarketRateResponse) -> Option<InstrumentResponse> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    // Parse tenor to get end date
    let end_date = calculate_end_date(&today, &rate.tenor)?;

    match rate.rate_type.as_str() {
        "Deposit" => Some(InstrumentResponse {
            instrument_type: "Deposit".to_string(),
            currency: rate.currency.clone(),
            start_date: today.clone(),
            end_date,
            rate: rate.value,
            parameters: HashMap::new(),
        }),
        "Swap" => Some(InstrumentResponse {
            instrument_type: "ParSwap".to_string(),
            currency: rate.currency.clone(),
            start_date: today.clone(),
            end_date,
            rate: rate.value,
            parameters: {
                let mut params = HashMap::new();
                params.insert("fixedFrequency".to_string(), serde_json::json!("Annual"));
                params.insert(
                    "floatFrequency".to_string(),
                    serde_json::json!("Quarterly"),
                );
                params
            },
        }),
        "Ois" => Some(InstrumentResponse {
            instrument_type: "OisSwap".to_string(),
            currency: rate.currency.clone(),
            start_date: today.clone(),
            end_date,
            rate: rate.value,
            parameters: {
                let mut params = HashMap::new();
                if let Some(ref index) = rate.rate_index {
                    params.insert("index".to_string(), serde_json::json!(index));
                }
                params
            },
        }),
        "FxSpot" | "Fra" | "Futures" => None, // Not directly mappable to instruments
        _ => None,
    }
}

/// Generates convention information for a given rate.
fn generate_convention_for_rate(rate: &MarketRateResponse) -> Option<ConventionResponse> {
    let convention_id = match (rate.currency.as_str(), rate.rate_type.as_str()) {
        ("USD", "Ois") => "USD-SOFR-OIS",
        ("USD", "Swap") => "USD-SOFR-SWAP",
        ("USD", "Deposit") => "USD-DEPO",
        ("EUR", "Ois") => "EUR-ESTR-OIS",
        ("EUR", "Swap") => "EUR-EURIBOR-SWAP",
        ("EUR", "Deposit") => "EUR-DEPO",
        ("JPY", "Ois") => "JPY-TONA-OIS",
        ("JPY", "Swap") => "JPY-TIBOR-SWAP",
        ("JPY", "Deposit") => "JPY-DEPO",
        (_, "FxSpot") => "FX-SPOT",
        _ => return None,
    };

    get_convention(convention_id)
}

/// Calculates end date from start date and tenor string.
fn calculate_end_date(start_date: &str, tenor: &str) -> Option<String> {
    // Parse start date
    let date = chrono::NaiveDate::parse_from_str(start_date, "%Y-%m-%d").ok()?;

    // Parse tenor
    let months = match tenor.to_uppercase().as_str() {
        "ON" | "O/N" => return Some(date.succ_opt()?.format("%Y-%m-%d").to_string()),
        "1W" | "1WK" => {
            return Some(date.checked_add_days(chrono::Days::new(7))?.format("%Y-%m-%d").to_string())
        }
        "2W" | "2WK" => {
            return Some(
                date.checked_add_days(chrono::Days::new(14))?
                    .format("%Y-%m-%d")
                    .to_string(),
            )
        }
        "1M" => 1,
        "2M" => 2,
        "3M" => 3,
        "6M" => 6,
        "9M" => 9,
        "1Y" => 12,
        "2Y" => 24,
        "3Y" => 36,
        "5Y" => 60,
        "7Y" => 84,
        "10Y" => 120,
        "15Y" => 180,
        "20Y" => 240,
        "30Y" => 360,
        "SPOT" => return Some(date.checked_add_days(chrono::Days::new(2))?.format("%Y-%m-%d").to_string()),
        _ => return None,
    };

    // Add months
    let end_date = date.checked_add_months(chrono::Months::new(months as u32))?;
    Some(end_date.format("%Y-%m-%d").to_string())
}

/// Simple pseudo-random number generator for perturbations.
fn pseudo_random(seed: i64) -> i64 {
    // LCG parameters
    let a: i64 = 1103515245;
    let c: i64 = 12345;
    let m: i64 = 2147483648;

    ((seed.wrapping_mul(a).wrapping_add(c)) % m).abs()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod market_data_cache_tests {
        use super::*;

        #[tokio::test]
        async fn test_cache_creates_sample_data() {
            let cache = MarketDataCache::new();
            let response = cache.get_rates(&MarketRateQuery::default()).await;

            // Should have a reasonable number of rates
            assert!(response.total_count > 20);
            assert!(!response.rates.is_empty());
        }

        #[tokio::test]
        async fn test_cache_filter_by_currency() {
            let cache = MarketDataCache::new();
            let query = MarketRateQuery {
                currency: Some("USD".to_string()),
                rate_type: None,
                index: None,
            };

            let response = cache.get_rates(&query).await;

            // All rates should be USD
            for rate in &response.rates {
                assert_eq!(rate.currency, "USD");
            }
        }

        #[tokio::test]
        async fn test_cache_filter_by_rate_type() {
            let cache = MarketDataCache::new();
            let query = MarketRateQuery {
                currency: None,
                rate_type: Some("Ois".to_string()),
                index: None,
            };

            let response = cache.get_rates(&query).await;

            for rate in &response.rates {
                assert_eq!(rate.rate_type, "Ois");
            }
        }

        #[tokio::test]
        async fn test_cache_filter_by_index() {
            let cache = MarketDataCache::new();
            let query = MarketRateQuery {
                currency: None,
                rate_type: None,
                index: Some("SOFR".to_string()),
            };

            let response = cache.get_rates(&query).await;

            for rate in &response.rates {
                assert_eq!(rate.rate_index, Some("SOFR".to_string()));
            }
        }

        #[tokio::test]
        async fn test_cache_get_rate_by_id() {
            let cache = MarketDataCache::new();

            let rate = cache.get_rate("USD-3M-DEPO").await;
            assert!(rate.is_some());

            let rate = rate.unwrap();
            assert_eq!(rate.currency, "USD");
            assert_eq!(rate.tenor, "3M");
            assert_eq!(rate.rate_type, "Deposit");
        }

        #[tokio::test]
        async fn test_cache_get_rate_detail() {
            let cache = MarketDataCache::new();

            let detail = cache.get_rate_detail("USD-5Y-SWAP").await;
            assert!(detail.is_some());

            let detail = detail.unwrap();
            assert_eq!(detail.rate.id, "USD-5Y-SWAP");
            assert!(detail.instrument.is_some());
            assert!(detail.convention.is_some());
        }

        #[tokio::test]
        async fn test_cache_refresh() {
            let cache = MarketDataCache::new();
            let initial_ts = cache.last_updated();

            // Wait a tiny bit to ensure timestamp changes
            tokio::time::sleep(Duration::from_millis(10)).await;

            cache.refresh().await;

            let new_ts = cache.last_updated();
            assert!(new_ts > initial_ts);
        }
    }

    mod rate_generation_tests {
        use super::*;

        #[test]
        fn test_usd_rates_generation() {
            let rates = generate_usd_rates(1700000000000);

            // Should have deposits
            let deposits: Vec<_> = rates.iter().filter(|r| r.rate_type == "Deposit").collect();
            assert!(!deposits.is_empty());

            // Should have OIS rates
            let ois: Vec<_> = rates.iter().filter(|r| r.rate_type == "Ois").collect();
            assert!(!ois.is_empty());

            // Should have swaps
            let swaps: Vec<_> = rates.iter().filter(|r| r.rate_type == "Swap").collect();
            assert!(!swaps.is_empty());
        }

        #[test]
        fn test_eur_rates_generation() {
            let rates = generate_eur_rates(1700000000000);

            // All should be EUR
            for rate in &rates {
                assert_eq!(rate.currency, "EUR");
            }

            // Should have ESTR index for OIS
            let ois_with_estr: Vec<_> = rates
                .iter()
                .filter(|r| r.rate_index == Some("ESTR".to_string()))
                .collect();
            assert!(!ois_with_estr.is_empty());
        }

        #[test]
        fn test_jpy_rates_generation() {
            let rates = generate_jpy_rates(1700000000000);

            // All should be JPY
            for rate in &rates {
                assert_eq!(rate.currency, "JPY");
            }

            // JPY can have negative deposit rates
            let deposits: Vec<_> = rates.iter().filter(|r| r.rate_type == "Deposit").collect();
            let has_negative = deposits.iter().any(|r| r.value < 0.0);
            assert!(has_negative);
        }

        #[test]
        fn test_fx_rates_generation() {
            let rates = generate_fx_rates(1700000000000);

            // All should be FxSpot
            for rate in &rates {
                assert_eq!(rate.rate_type, "FxSpot");
                assert_eq!(rate.tenor, "SPOT");
            }

            // Should have major pairs
            let eurusd = rates.iter().find(|r| r.id == "EURUSD-SPOT");
            assert!(eurusd.is_some());

            let usdjpy = rates.iter().find(|r| r.id == "USDJPY-SPOT");
            assert!(usdjpy.is_some());
        }
    }

    mod convention_tests {
        use super::*;

        #[test]
        fn test_get_conventions_list() {
            let list = get_conventions_list();
            assert!(!list.conventions.is_empty());

            // Should have USD, EUR, JPY conventions
            let currencies: Vec<_> = list.conventions.iter().map(|c| &c.currency).collect();
            assert!(currencies.contains(&&"USD".to_string()));
            assert!(currencies.contains(&&"EUR".to_string()));
            assert!(currencies.contains(&&"JPY".to_string()));
        }

        #[test]
        fn test_get_convention_by_id() {
            let conv = get_convention("USD-SOFR-OIS");
            assert!(conv.is_some());

            let conv = conv.unwrap();
            assert_eq!(conv.convention_type, "OisConvention");
            assert!(!conv.fields.is_empty());

            // Should have SOFR index
            let index_field = conv.fields.iter().find(|f| f.label == "Index");
            assert!(index_field.is_some());
            assert_eq!(index_field.unwrap().value, "SOFR");
        }

        #[test]
        fn test_get_convention_not_found() {
            let conv = get_convention("INVALID-CONVENTION");
            assert!(conv.is_none());
        }
    }

    mod instrument_generation_tests {
        use super::*;

        #[test]
        fn test_deposit_instrument() {
            let rate = MarketRateResponse {
                id: "USD-3M-DEPO".to_string(),
                currency: "USD".to_string(),
                tenor: "3M".to_string(),
                rate_type: "Deposit".to_string(),
                value: 0.05,
                quote_type: "Mid".to_string(),
                timestamp: 1700000000000,
                source: "Bloomberg".to_string(),
                is_stale: false,
                rate_index: None,
            };

            let instrument = generate_instrument_for_rate(&rate);
            assert!(instrument.is_some());

            let instrument = instrument.unwrap();
            assert_eq!(instrument.instrument_type, "Deposit");
            assert_eq!(instrument.currency, "USD");
        }

        #[test]
        fn test_swap_instrument() {
            let rate = MarketRateResponse {
                id: "USD-5Y-SWAP".to_string(),
                currency: "USD".to_string(),
                tenor: "5Y".to_string(),
                rate_type: "Swap".to_string(),
                value: 0.04,
                quote_type: "Mid".to_string(),
                timestamp: 1700000000000,
                source: "Bloomberg".to_string(),
                is_stale: false,
                rate_index: Some("SOFR".to_string()),
            };

            let instrument = generate_instrument_for_rate(&rate);
            assert!(instrument.is_some());

            let instrument = instrument.unwrap();
            assert_eq!(instrument.instrument_type, "ParSwap");
            assert!(instrument.parameters.contains_key("fixedFrequency"));
        }

        #[test]
        fn test_fx_spot_no_instrument() {
            let rate = MarketRateResponse {
                id: "EURUSD-SPOT".to_string(),
                currency: "EUR".to_string(),
                tenor: "SPOT".to_string(),
                rate_type: "FxSpot".to_string(),
                value: 1.0875,
                quote_type: "Mid".to_string(),
                timestamp: 1700000000000,
                source: "Reuters".to_string(),
                is_stale: false,
                rate_index: None,
            };

            let instrument = generate_instrument_for_rate(&rate);
            assert!(instrument.is_none());
        }
    }

    mod date_calculation_tests {
        use super::*;

        #[test]
        fn test_calculate_end_date_months() {
            let end = calculate_end_date("2024-01-15", "3M");
            assert_eq!(end, Some("2024-04-15".to_string()));

            let end = calculate_end_date("2024-01-15", "1Y");
            assert_eq!(end, Some("2025-01-15".to_string()));

            let end = calculate_end_date("2024-01-15", "5Y");
            assert_eq!(end, Some("2029-01-15".to_string()));
        }

        #[test]
        fn test_calculate_end_date_weeks() {
            let end = calculate_end_date("2024-01-15", "1W");
            assert_eq!(end, Some("2024-01-22".to_string()));

            let end = calculate_end_date("2024-01-15", "2W");
            assert_eq!(end, Some("2024-01-29".to_string()));
        }

        #[test]
        fn test_calculate_end_date_overnight() {
            let end = calculate_end_date("2024-01-15", "ON");
            assert_eq!(end, Some("2024-01-16".to_string()));
        }

        #[test]
        fn test_calculate_end_date_spot() {
            let end = calculate_end_date("2024-01-15", "SPOT");
            assert_eq!(end, Some("2024-01-17".to_string()));
        }
    }
}
