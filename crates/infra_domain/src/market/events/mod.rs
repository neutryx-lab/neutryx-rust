//! Market event types and definitions.
//!
//! This module provides types for economic calendar and market event tracking:
//!
//! - [`EventType`]: Classification of market events
//! - [`EventImportance`]: Importance/impact level of events
//! - [`CentralBank`]: Central bank identifier
//! - [`MarketEvent`]: Complete market event structure
//!
//! # Examples
//!
//! ```
//! use infra_master::market::events::{EventType, EventImportance, MarketEvent, CentralBank};
//!
//! // Create a central bank meeting event
//! let fomc = MarketEvent::new(
//!     "FOMC-2024-03",
//!     EventType::CentralBankMeeting,
//!     "FOMC Interest Rate Decision",
//!     "2024-03-20",
//!     EventImportance::Critical,
//!     "Bloomberg",
//! )
//! .with_currency("USD")
//! .with_central_bank(CentralBank::fed())
//! .with_time("14:00")
//! .with_timezone("America/New_York");
//!
//! assert!(fomc.is_central_bank_event());
//! assert!(fomc.is_high_impact());
//! ```

mod central_bank;
mod event_type;
mod importance;
mod market_event;

pub use central_bank::CentralBank;
pub use event_type::EventType;
pub use importance::EventImportance;
pub use market_event::MarketEvent;
