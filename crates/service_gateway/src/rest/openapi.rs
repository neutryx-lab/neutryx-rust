//! OpenAPI specification and Swagger UI integration.

use utoipa::OpenApi;

/// Auto-generated OpenAPI documentation for the Neutryx REST API.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Neutryx Pricing API",
        version = "0.1.0",
        description = "Production-grade derivatives pricing library REST API"
    ),
    paths(
        super::handlers::price_instrument,
        super::handlers::price_portfolio,
        super::handlers::build_curve,
        super::handlers::get_discount_factor,
        super::handlers::get_forward_rate,
        super::handlers::get_forward_swap_rates,
    ),
    components(schemas(
        // Pricing
        super::dto::InstrumentType,
        super::dto::PricingRequest,
        super::dto::PricingResponse,
        super::dto::GreeksResponse,
        super::dto::PortfolioPricingRequest,
        super::dto::PortfolioPricingResponse,
        super::dto::PortfolioInstrumentResult,
        super::dto::HealthResponse,
        // Curves
        super::dto::BootstrapMethod,
        super::dto::CurveInstrumentInput,
        super::dto::CurveBuildRequest,
        super::dto::CurveBuildResponse,
        super::dto::CurvePillar,
        super::dto::ForwardRatePoint,
        super::dto::ChartGridPoint,
        super::dto::JacobianData,
        super::dto::DiscountFactorRequest,
        super::dto::DiscountFactorResponse,
        super::dto::ForwardRateRequest,
        super::dto::ForwardRateResponse,
        super::dto::ForwardSwapRateRequest,
        super::dto::ForwardSwapRateResponse,
    )),
    tags(
        (name = "pricing", description = "Instrument pricing endpoints"),
        (name = "curves", description = "Yield curve building and query endpoints"),
    )
)]
pub struct ApiDoc;
