//! Full-valuation Monte Carlo XVA engine with 3-tier portfolio hierarchy.

mod aggregator;
mod calibration;
mod config;
mod error;
mod hierarchy;
mod risk_indicators;
mod simulator;

pub use aggregator::ExposureAggregator;
pub use calibration::{CalibrationDag, CalibrationEntry, CalibrationSolver, GlobalCalibrationTable};
pub use config::XvaEngineConfig;
pub use error::XvaEngineError;
pub use hierarchy::{
    IsdaAgreement, OtherExposurePaths, VmCsaNode, XvaCounterparty, XvaHierarchy,
};
pub use risk_indicators::XvaRiskIndicators;
pub use simulator::{XvaSimulationResult, XvaSimulator};
