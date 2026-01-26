//! Events API handlers for the Market Data Viewer WebApp.
//!
//! Provides REST API endpoints for market events:
//! - Central Bank meetings
//! - Economic data releases
//! - Market holidays
//! - News and announcements

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{Datelike, Duration, NaiveDate, Utc};
use serde_json::json;

use super::events_types::*;

// =============================================================================
// Sample Data Generation
// =============================================================================

/// Get list of major central banks.
fn get_central_banks() -> Vec<CentralBank> {
    vec![
        CentralBank {
            code: "FED".to_string(),
            name: "Federal Reserve".to_string(),
            currency: "USD".to_string(),
            region: "United States".to_string(),
        },
        CentralBank {
            code: "ECB".to_string(),
            name: "European Central Bank".to_string(),
            currency: "EUR".to_string(),
            region: "Eurozone".to_string(),
        },
        CentralBank {
            code: "BOJ".to_string(),
            name: "Bank of Japan".to_string(),
            currency: "JPY".to_string(),
            region: "Japan".to_string(),
        },
        CentralBank {
            code: "BOE".to_string(),
            name: "Bank of England".to_string(),
            currency: "GBP".to_string(),
            region: "United Kingdom".to_string(),
        },
        CentralBank {
            code: "SNB".to_string(),
            name: "Swiss National Bank".to_string(),
            currency: "CHF".to_string(),
            region: "Switzerland".to_string(),
        },
        CentralBank {
            code: "RBA".to_string(),
            name: "Reserve Bank of Australia".to_string(),
            currency: "AUD".to_string(),
            region: "Australia".to_string(),
        },
        CentralBank {
            code: "BOC".to_string(),
            name: "Bank of Canada".to_string(),
            currency: "CAD".to_string(),
            region: "Canada".to_string(),
        },
        CentralBank {
            code: "RBNZ".to_string(),
            name: "Reserve Bank of New Zealand".to_string(),
            currency: "NZD".to_string(),
            region: "New Zealand".to_string(),
        },
    ]
}

/// Generate sample central bank meeting dates for 2025-2026.
fn generate_cb_meetings() -> Vec<MarketEvent> {
    let mut events = Vec::new();
    let central_banks = get_central_banks();

    // FOMC (Federal Reserve) - 8 meetings per year
    let fomc_dates_2025 = [
        "2025-01-29", "2025-03-19", "2025-05-07", "2025-06-18",
        "2025-07-30", "2025-09-17", "2025-11-05", "2025-12-17",
    ];
    let fomc_dates_2026 = [
        "2026-01-28", "2026-03-18", "2026-05-06", "2026-06-17",
        "2026-07-29", "2026-09-16", "2026-11-04", "2026-12-16",
    ];

    let fed = central_banks.iter().find(|b| b.code == "FED").unwrap();
    for (i, date) in fomc_dates_2025.iter().chain(fomc_dates_2026.iter()).enumerate() {
        events.push(MarketEvent {
            id: format!("FED-{}", i + 1),
            event_type: EventType::CentralBankMeeting,
            title: "FOMC Interest Rate Decision".to_string(),
            description: Some("Federal Open Market Committee policy rate decision and statement".to_string()),
            date: date.to_string(),
            time: Some("14:00".to_string()),
            timezone: Some("America/New_York".to_string()),
            currency: Some("USD".to_string()),
            region: Some("United States".to_string()),
            importance: EventImportance::Critical,
            central_bank: Some(fed.clone()),
            previous: None,
            forecast: None,
            actual: None,
            source: "Federal Reserve".to_string(),
            tags: vec!["fomc".to_string(), "rates".to_string(), "policy".to_string()],
        });
    }

    // ECB - 8 meetings per year
    let ecb_dates_2025 = [
        "2025-01-30", "2025-03-06", "2025-04-17", "2025-06-05",
        "2025-07-17", "2025-09-11", "2025-10-30", "2025-12-11",
    ];
    let ecb_dates_2026 = [
        "2026-01-22", "2026-03-05", "2026-04-16", "2026-06-04",
        "2026-07-16", "2026-09-10", "2026-10-29", "2026-12-10",
    ];

    let ecb = central_banks.iter().find(|b| b.code == "ECB").unwrap();
    for (i, date) in ecb_dates_2025.iter().chain(ecb_dates_2026.iter()).enumerate() {
        events.push(MarketEvent {
            id: format!("ECB-{}", i + 1),
            event_type: EventType::CentralBankMeeting,
            title: "ECB Interest Rate Decision".to_string(),
            description: Some("European Central Bank monetary policy decision and press conference".to_string()),
            date: date.to_string(),
            time: Some("13:45".to_string()),
            timezone: Some("Europe/Frankfurt".to_string()),
            currency: Some("EUR".to_string()),
            region: Some("Eurozone".to_string()),
            importance: EventImportance::Critical,
            central_bank: Some(ecb.clone()),
            previous: None,
            forecast: None,
            actual: None,
            source: "European Central Bank".to_string(),
            tags: vec!["ecb".to_string(), "rates".to_string(), "policy".to_string()],
        });
    }

    // BOJ - 8 meetings per year
    let boj_dates_2025 = [
        "2025-01-24", "2025-03-14", "2025-04-25", "2025-06-13",
        "2025-07-31", "2025-09-19", "2025-10-31", "2025-12-19",
    ];
    let boj_dates_2026 = [
        "2026-01-23", "2026-03-13", "2026-04-24", "2026-06-12",
        "2026-07-17", "2026-09-18", "2026-10-30", "2026-12-18",
    ];

    let boj = central_banks.iter().find(|b| b.code == "BOJ").unwrap();
    for (i, date) in boj_dates_2025.iter().chain(boj_dates_2026.iter()).enumerate() {
        events.push(MarketEvent {
            id: format!("BOJ-{}", i + 1),
            event_type: EventType::CentralBankMeeting,
            title: "BOJ Monetary Policy Decision".to_string(),
            description: Some("Bank of Japan monetary policy meeting and outlook report".to_string()),
            date: date.to_string(),
            time: Some("12:00".to_string()),
            timezone: Some("Asia/Tokyo".to_string()),
            currency: Some("JPY".to_string()),
            region: Some("Japan".to_string()),
            importance: EventImportance::Critical,
            central_bank: Some(boj.clone()),
            previous: None,
            forecast: None,
            actual: None,
            source: "Bank of Japan".to_string(),
            tags: vec!["boj".to_string(), "rates".to_string(), "policy".to_string()],
        });
    }

    // BOE - 8 meetings per year
    let boe_dates_2025 = [
        "2025-02-06", "2025-03-20", "2025-05-08", "2025-06-19",
        "2025-08-07", "2025-09-18", "2025-11-06", "2025-12-18",
    ];
    let boe_dates_2026 = [
        "2026-02-05", "2026-03-19", "2026-05-07", "2026-06-18",
        "2026-08-06", "2026-09-17", "2026-11-05", "2026-12-17",
    ];

    let boe = central_banks.iter().find(|b| b.code == "BOE").unwrap();
    for (i, date) in boe_dates_2025.iter().chain(boe_dates_2026.iter()).enumerate() {
        events.push(MarketEvent {
            id: format!("BOE-{}", i + 1),
            event_type: EventType::CentralBankMeeting,
            title: "BOE Interest Rate Decision".to_string(),
            description: Some("Bank of England Monetary Policy Committee rate decision".to_string()),
            date: date.to_string(),
            time: Some("12:00".to_string()),
            timezone: Some("Europe/London".to_string()),
            currency: Some("GBP".to_string()),
            region: Some("United Kingdom".to_string()),
            importance: EventImportance::Critical,
            central_bank: Some(boe.clone()),
            previous: None,
            forecast: None,
            actual: None,
            source: "Bank of England".to_string(),
            tags: vec!["boe".to_string(), "rates".to_string(), "policy".to_string()],
        });
    }

    events
}

/// Generate sample economic data release events.
fn generate_economic_releases() -> Vec<MarketEvent> {
    let mut events = Vec::new();
    let today = Utc::now().date_naive();

    // US Non-Farm Payrolls (first Friday of each month)
    for month_offset in 0..12 {
        let target_month = today + Duration::days(month_offset * 30);
        let first_day = NaiveDate::from_ymd_opt(target_month.year(), target_month.month(), 1).unwrap();
        let days_to_friday = (5 - first_day.weekday().num_days_from_monday() + 7) % 7;
        let nfp_date = first_day + Duration::days(days_to_friday as i64);

        events.push(MarketEvent {
            id: format!("NFP-{}", month_offset + 1),
            event_type: EventType::EconomicRelease,
            title: "US Non-Farm Payrolls".to_string(),
            description: Some("Monthly employment report from the Bureau of Labor Statistics".to_string()),
            date: nfp_date.format("%Y-%m-%d").to_string(),
            time: Some("08:30".to_string()),
            timezone: Some("America/New_York".to_string()),
            currency: Some("USD".to_string()),
            region: Some("United States".to_string()),
            importance: EventImportance::Critical,
            central_bank: None,
            previous: Some("+175K".to_string()),
            forecast: Some("+180K".to_string()),
            actual: None,
            source: "BLS".to_string(),
            tags: vec!["employment".to_string(), "labor".to_string(), "nfp".to_string()],
        });
    }

    // US CPI (mid-month)
    for month_offset in 0..12 {
        let target_month = today + Duration::days(month_offset * 30);
        let cpi_date = NaiveDate::from_ymd_opt(target_month.year(), target_month.month(), 13).unwrap();

        events.push(MarketEvent {
            id: format!("CPI-US-{}", month_offset + 1),
            event_type: EventType::EconomicRelease,
            title: "US Consumer Price Index".to_string(),
            description: Some("Monthly inflation data measuring changes in consumer prices".to_string()),
            date: cpi_date.format("%Y-%m-%d").to_string(),
            time: Some("08:30".to_string()),
            timezone: Some("America/New_York".to_string()),
            currency: Some("USD".to_string()),
            region: Some("United States".to_string()),
            importance: EventImportance::High,
            central_bank: None,
            previous: Some("2.9% YoY".to_string()),
            forecast: Some("2.8% YoY".to_string()),
            actual: None,
            source: "BLS".to_string(),
            tags: vec!["inflation".to_string(), "cpi".to_string(), "prices".to_string()],
        });
    }

    // US GDP (quarterly)
    let gdp_dates = ["2025-01-30", "2025-04-30", "2025-07-30", "2025-10-30", "2026-01-29", "2026-04-29"];
    for (i, date) in gdp_dates.iter().enumerate() {
        events.push(MarketEvent {
            id: format!("GDP-US-{}", i + 1),
            event_type: EventType::EconomicRelease,
            title: "US GDP (Advance)".to_string(),
            description: Some("Quarterly Gross Domestic Product advance estimate".to_string()),
            date: date.to_string(),
            time: Some("08:30".to_string()),
            timezone: Some("America/New_York".to_string()),
            currency: Some("USD".to_string()),
            region: Some("United States".to_string()),
            importance: EventImportance::High,
            central_bank: None,
            previous: Some("2.8% QoQ".to_string()),
            forecast: Some("2.5% QoQ".to_string()),
            actual: None,
            source: "BEA".to_string(),
            tags: vec!["growth".to_string(), "gdp".to_string(), "economy".to_string()],
        });
    }

    events
}

/// Generate sample market holidays.
fn generate_holidays() -> Vec<MarketEvent> {
    vec![
        // US Holidays 2025
        MarketEvent {
            id: "HOL-US-1".to_string(),
            event_type: EventType::Holiday,
            title: "New Year's Day".to_string(),
            description: Some("US markets closed".to_string()),
            date: "2025-01-01".to_string(),
            time: None,
            timezone: Some("America/New_York".to_string()),
            currency: Some("USD".to_string()),
            region: Some("United States".to_string()),
            importance: EventImportance::Medium,
            central_bank: None,
            previous: None,
            forecast: None,
            actual: None,
            source: "NYSE".to_string(),
            tags: vec!["holiday".to_string(), "market-closed".to_string()],
        },
        MarketEvent {
            id: "HOL-US-2".to_string(),
            event_type: EventType::Holiday,
            title: "Martin Luther King Jr. Day".to_string(),
            description: Some("US markets closed".to_string()),
            date: "2025-01-20".to_string(),
            time: None,
            timezone: Some("America/New_York".to_string()),
            currency: Some("USD".to_string()),
            region: Some("United States".to_string()),
            importance: EventImportance::Medium,
            central_bank: None,
            previous: None,
            forecast: None,
            actual: None,
            source: "NYSE".to_string(),
            tags: vec!["holiday".to_string(), "market-closed".to_string()],
        },
        MarketEvent {
            id: "HOL-US-3".to_string(),
            event_type: EventType::Holiday,
            title: "Presidents' Day".to_string(),
            description: Some("US markets closed".to_string()),
            date: "2025-02-17".to_string(),
            time: None,
            timezone: Some("America/New_York".to_string()),
            currency: Some("USD".to_string()),
            region: Some("United States".to_string()),
            importance: EventImportance::Medium,
            central_bank: None,
            previous: None,
            forecast: None,
            actual: None,
            source: "NYSE".to_string(),
            tags: vec!["holiday".to_string(), "market-closed".to_string()],
        },
        MarketEvent {
            id: "HOL-US-4".to_string(),
            event_type: EventType::Holiday,
            title: "Good Friday".to_string(),
            description: Some("US markets closed".to_string()),
            date: "2025-04-18".to_string(),
            time: None,
            timezone: Some("America/New_York".to_string()),
            currency: Some("USD".to_string()),
            region: Some("United States".to_string()),
            importance: EventImportance::Medium,
            central_bank: None,
            previous: None,
            forecast: None,
            actual: None,
            source: "NYSE".to_string(),
            tags: vec!["holiday".to_string(), "market-closed".to_string()],
        },
        MarketEvent {
            id: "HOL-US-5".to_string(),
            event_type: EventType::Holiday,
            title: "Memorial Day".to_string(),
            description: Some("US markets closed".to_string()),
            date: "2025-05-26".to_string(),
            time: None,
            timezone: Some("America/New_York".to_string()),
            currency: Some("USD".to_string()),
            region: Some("United States".to_string()),
            importance: EventImportance::Medium,
            central_bank: None,
            previous: None,
            forecast: None,
            actual: None,
            source: "NYSE".to_string(),
            tags: vec!["holiday".to_string(), "market-closed".to_string()],
        },
        MarketEvent {
            id: "HOL-US-6".to_string(),
            event_type: EventType::Holiday,
            title: "Independence Day".to_string(),
            description: Some("US markets closed".to_string()),
            date: "2025-07-04".to_string(),
            time: None,
            timezone: Some("America/New_York".to_string()),
            currency: Some("USD".to_string()),
            region: Some("United States".to_string()),
            importance: EventImportance::Medium,
            central_bank: None,
            previous: None,
            forecast: None,
            actual: None,
            source: "NYSE".to_string(),
            tags: vec!["holiday".to_string(), "market-closed".to_string()],
        },
        MarketEvent {
            id: "HOL-US-7".to_string(),
            event_type: EventType::Holiday,
            title: "Labor Day".to_string(),
            description: Some("US markets closed".to_string()),
            date: "2025-09-01".to_string(),
            time: None,
            timezone: Some("America/New_York".to_string()),
            currency: Some("USD".to_string()),
            region: Some("United States".to_string()),
            importance: EventImportance::Medium,
            central_bank: None,
            previous: None,
            forecast: None,
            actual: None,
            source: "NYSE".to_string(),
            tags: vec!["holiday".to_string(), "market-closed".to_string()],
        },
        MarketEvent {
            id: "HOL-US-8".to_string(),
            event_type: EventType::Holiday,
            title: "Thanksgiving Day".to_string(),
            description: Some("US markets closed".to_string()),
            date: "2025-11-27".to_string(),
            time: None,
            timezone: Some("America/New_York".to_string()),
            currency: Some("USD".to_string()),
            region: Some("United States".to_string()),
            importance: EventImportance::Medium,
            central_bank: None,
            previous: None,
            forecast: None,
            actual: None,
            source: "NYSE".to_string(),
            tags: vec!["holiday".to_string(), "market-closed".to_string()],
        },
        MarketEvent {
            id: "HOL-US-9".to_string(),
            event_type: EventType::Holiday,
            title: "Christmas Day".to_string(),
            description: Some("US markets closed".to_string()),
            date: "2025-12-25".to_string(),
            time: None,
            timezone: Some("America/New_York".to_string()),
            currency: Some("USD".to_string()),
            region: Some("United States".to_string()),
            importance: EventImportance::Medium,
            central_bank: None,
            previous: None,
            forecast: None,
            actual: None,
            source: "NYSE".to_string(),
            tags: vec!["holiday".to_string(), "market-closed".to_string()],
        },
        // Japan Holidays 2025
        MarketEvent {
            id: "HOL-JP-1".to_string(),
            event_type: EventType::Holiday,
            title: "Coming of Age Day".to_string(),
            description: Some("Japan markets closed".to_string()),
            date: "2025-01-13".to_string(),
            time: None,
            timezone: Some("Asia/Tokyo".to_string()),
            currency: Some("JPY".to_string()),
            region: Some("Japan".to_string()),
            importance: EventImportance::Medium,
            central_bank: None,
            previous: None,
            forecast: None,
            actual: None,
            source: "TSE".to_string(),
            tags: vec!["holiday".to_string(), "market-closed".to_string()],
        },
        MarketEvent {
            id: "HOL-JP-2".to_string(),
            event_type: EventType::Holiday,
            title: "Golden Week".to_string(),
            description: Some("Japan markets closed for Golden Week holidays".to_string()),
            date: "2025-05-03".to_string(),
            time: None,
            timezone: Some("Asia/Tokyo".to_string()),
            currency: Some("JPY".to_string()),
            region: Some("Japan".to_string()),
            importance: EventImportance::High,
            central_bank: None,
            previous: None,
            forecast: None,
            actual: None,
            source: "TSE".to_string(),
            tags: vec!["holiday".to_string(), "market-closed".to_string(), "golden-week".to_string()],
        },
        // UK Holidays 2025
        MarketEvent {
            id: "HOL-UK-1".to_string(),
            event_type: EventType::Holiday,
            title: "Early May Bank Holiday".to_string(),
            description: Some("UK markets closed".to_string()),
            date: "2025-05-05".to_string(),
            time: None,
            timezone: Some("Europe/London".to_string()),
            currency: Some("GBP".to_string()),
            region: Some("United Kingdom".to_string()),
            importance: EventImportance::Medium,
            central_bank: None,
            previous: None,
            forecast: None,
            actual: None,
            source: "LSE".to_string(),
            tags: vec!["holiday".to_string(), "market-closed".to_string()],
        },
        MarketEvent {
            id: "HOL-UK-2".to_string(),
            event_type: EventType::Holiday,
            title: "Spring Bank Holiday".to_string(),
            description: Some("UK markets closed".to_string()),
            date: "2025-05-26".to_string(),
            time: None,
            timezone: Some("Europe/London".to_string()),
            currency: Some("GBP".to_string()),
            region: Some("United Kingdom".to_string()),
            importance: EventImportance::Medium,
            central_bank: None,
            previous: None,
            forecast: None,
            actual: None,
            source: "LSE".to_string(),
            tags: vec!["holiday".to_string(), "market-closed".to_string()],
        },
    ]
}

/// Get all sample events.
fn get_all_events() -> Vec<MarketEvent> {
    let mut events = Vec::new();
    events.extend(generate_cb_meetings());
    events.extend(generate_economic_releases());
    events.extend(generate_holidays());

    // Sort by date
    events.sort_by(|a, b| a.date.cmp(&b.date));
    events
}

// =============================================================================
// API Handlers
// =============================================================================

/// GET /api/events
/// List market events with optional filters.
pub async fn get_events(Query(query): Query<EventsQuery>) -> impl IntoResponse {
    let mut events = get_all_events();

    // Apply filters
    if let Some(ref event_type) = query.event_type {
        let type_lower = event_type.to_lowercase();
        events.retain(|e| {
            let type_str = format!("{:?}", e.event_type).to_lowercase();
            type_str.contains(&type_lower) || type_lower.contains(&type_str.replace("_", ""))
        });
    }

    if let Some(ref currency) = query.currency {
        let currency_upper = currency.to_uppercase();
        events.retain(|e| {
            e.currency.as_ref().map(|c| c == &currency_upper).unwrap_or(false)
        });
    }

    if let Some(ref region) = query.region {
        let region_lower = region.to_lowercase();
        events.retain(|e| {
            e.region.as_ref().map(|r| r.to_lowercase().contains(&region_lower)).unwrap_or(false)
        });
    }

    if let Some(ref importance) = query.importance {
        let imp_lower = importance.to_lowercase();
        events.retain(|e| {
            let imp_str = format!("{:?}", e.importance).to_lowercase();
            imp_str == imp_lower
        });
    }

    if let Some(ref start_date) = query.start_date {
        events.retain(|e| e.date >= *start_date);
    }

    if let Some(ref end_date) = query.end_date {
        events.retain(|e| e.date <= *end_date);
    }

    let total_count = events.len();

    // Apply limit
    if let Some(limit) = query.limit {
        events.truncate(limit);
    }

    Json(EventsResponse {
        events,
        total_count,
        last_updated: chrono::Utc::now().timestamp_millis(),
    })
}

/// GET /api/events/{id}
/// Get single event details.
pub async fn get_event_detail(Path(id): Path<String>) -> impl IntoResponse {
    let events = get_all_events();

    match events.iter().find(|e| e.id == id) {
        Some(event) => {
            // Find related events (same type and currency)
            let related: Vec<MarketEvent> = events
                .iter()
                .filter(|e| {
                    e.id != id
                        && e.event_type == event.event_type
                        && e.currency == event.currency
                })
                .take(5)
                .cloned()
                .collect();

            Json(EventDetailResponse {
                event: event.clone(),
                related_events: related,
            })
            .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "not_found",
                "message": format!("Event {} not found", id)
            })),
        )
            .into_response(),
    }
}

/// GET /api/events/types
/// List available event types.
pub async fn get_event_types() -> impl IntoResponse {
    let events = get_all_events();

    let types = vec![
        EventType::CentralBankMeeting,
        EventType::EconomicRelease,
        EventType::Holiday,
        EventType::News,
        EventType::Expiry,
        EventType::Other,
    ];

    let type_infos: Vec<EventTypeInfo> = types
        .into_iter()
        .map(|t| {
            let count = events.iter().filter(|e| e.event_type == t).count();
            EventTypeInfo {
                value: t,
                display_name: t.display_name().to_string(),
                icon: t.icon().to_string(),
                count,
            }
        })
        .collect();

    Json(EventTypesResponse { types: type_infos })
}

/// GET /api/events/central-banks
/// List central banks.
pub async fn get_central_banks_list() -> impl IntoResponse {
    Json(CentralBanksResponse {
        central_banks: get_central_banks(),
    })
}
