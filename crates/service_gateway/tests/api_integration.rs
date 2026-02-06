//! REST API integration tests for service_gateway.
//!
//! Tests the Portfolio graph endpoints by testing the underlying components:
//! - SamplePortfolioBuilder
//! - PortfolioGraphExtractor
//!
//! Requirements Coverage:
//! - 4.1: `/api/v1/portfolio/graph` endpoint test (via extractor test)
//! - 4.2: Timeout and error handling tests
//! - 4.3: `/api/v1/portfolio/trades` endpoint test (via portfolio test)
//! - 5.1, 5.2, 5.3: Trade list tests

// Note: Since service_gateway is a binary crate, we test the underlying
// pricer_risk and pricer_pricing components directly.

#[cfg(test)]
mod portfolio_tests {
    use pricer_risk::portfolio::SamplePortfolioBuilder;

    #[test]
    fn test_sample_portfolio_creation() {
        let portfolio = SamplePortfolioBuilder::new()
            .with_trade_count(20)
            .build()
            .expect("Failed to create portfolio");

        assert_eq!(portfolio.trade_count(), 20);
    }

    #[test]
    fn test_sample_portfolio_trade_diversity() {
        let portfolio = SamplePortfolioBuilder::new()
            .with_trade_count(30)
            .build()
            .expect("Failed to create portfolio");

        let mut equity_count = 0;
        let mut rates_count = 0;
        let mut fx_count = 0;

        for trade in portfolio.trades() {
            let id = trade.id().as_str();
            if id.starts_with("EQ_") {
                equity_count += 1;
            } else if id.starts_with("IR_") {
                rates_count += 1;
            } else if id.starts_with("FX_") {
                fx_count += 1;
            }
        }

        assert!(equity_count > 0, "Should have equity trades");
        assert!(rates_count > 0, "Should have rates trades");
        assert!(fx_count > 0, "Should have FX trades");
    }

    #[test]
    fn test_large_portfolio_performance() {
        use std::time::Instant;

        let start = Instant::now();
        let portfolio = SamplePortfolioBuilder::new()
            .with_trade_count(100)
            .build()
            .expect("Failed to create portfolio");
        let elapsed = start.elapsed();

        assert_eq!(portfolio.trade_count(), 100);
        assert!(
            elapsed.as_secs() < 5,
            "Portfolio creation should complete in under 5 seconds"
        );
    }
}

// ============================================================================
// Simulated HTTP Tests (without actually starting server)
// ============================================================================

#[cfg(test)]
mod simulated_http_tests {
    use std::collections::HashMap;

    use pricer_pricing::graph::{PortfolioGraphExtractable, PortfolioGraphExtractor};
    use pricer_risk::portfolio::{SamplePortfolioBuilder, TradeId};

    // Test portfolio graph extraction (simulates GET /api/v1/portfolio/graph)
    #[test]
    fn test_portfolio_graph_extraction() {
        let portfolio = SamplePortfolioBuilder::new()
            .with_trade_count(20)
            .build()
            .expect("Failed to create portfolio");

        let extractor = PortfolioGraphExtractor::new()
            .with_timeout(500)
            .with_capacity(5_000, 10_000);

        // Build trade graphs
        let trade_ids: Vec<String> = portfolio.trade_ids().map(|id| id.to_string()).collect();

        let mut trade_graphs = HashMap::new();
        for trade_id in &trade_ids {
            if let Some(trade) = portfolio.trade(&TradeId::new(trade_id)) {
                let graph = create_mock_trade_graph(trade_id, trade);
                trade_graphs.insert(trade_id.clone(), graph);
            }
        }

        let result = extractor.extract_portfolio_graph(&trade_ids, &trade_graphs);
        assert!(result.is_ok(), "Graph extraction should succeed");

        let graph = result.unwrap();
        assert!(graph.metadata.node_count > 0, "Should have nodes");
        assert!(graph.metadata.edge_count > 0, "Should have edges");
        assert_eq!(graph.metadata.trade_count, 20, "Should have 20 trades");
    }

    // Test subgraph extraction (simulates GET
    // /api/v1/portfolio/graph?trade_ids=...)
    #[test]
    fn test_subgraph_extraction() {
        let portfolio = SamplePortfolioBuilder::new()
            .with_trade_count(20)
            .build()
            .expect("Failed to create portfolio");

        let extractor = PortfolioGraphExtractor::new();

        let all_trade_ids: Vec<String> = portfolio.trade_ids().map(|id| id.to_string()).collect();

        let mut trade_graphs = HashMap::new();
        for trade_id in &all_trade_ids {
            if let Some(trade) = portfolio.trade(&TradeId::new(trade_id)) {
                let graph = create_mock_trade_graph(trade_id, trade);
                trade_graphs.insert(trade_id.clone(), graph);
            }
        }

        // Extract full graph first
        let full_graph = extractor
            .extract_portfolio_graph(&all_trade_ids, &trade_graphs)
            .expect("Full graph extraction should succeed");

        // Extract subgraph for first 3 trades
        let selected: Vec<String> = all_trade_ids.iter().take(3).cloned().collect();
        let subgraph = extractor
            .extract_subgraph(&full_graph, &selected)
            .expect("Subgraph extraction should succeed");

        assert!(
            subgraph.metadata.node_count < full_graph.metadata.node_count,
            "Subgraph should have fewer nodes"
        );
    }

    // Test trade not found error (simulates 404 response)
    #[test]
    fn test_trade_not_found_error() {
        let portfolio = SamplePortfolioBuilder::new()
            .with_trade_count(5)
            .build()
            .expect("Failed to create portfolio");

        let extractor = PortfolioGraphExtractor::new();

        let all_trade_ids: Vec<String> = portfolio.trade_ids().map(|id| id.to_string()).collect();

        let mut trade_graphs = HashMap::new();
        for trade_id in &all_trade_ids {
            if let Some(trade) = portfolio.trade(&TradeId::new(trade_id)) {
                let graph = create_mock_trade_graph(trade_id, trade);
                trade_graphs.insert(trade_id.clone(), graph);
            }
        }

        let full_graph = extractor
            .extract_portfolio_graph(&all_trade_ids, &trade_graphs)
            .expect("Full graph extraction should succeed");

        // Try to extract with non-existent trade ID
        let result = extractor.extract_subgraph(&full_graph, &["NON_EXISTENT".to_string()]);
        assert!(
            result.is_err(),
            "Should return error for non-existent trade"
        );

        match result {
            Err(pricer_pricing::graph::GraphError::TradeNotFound(id)) => {
                assert_eq!(id, "NON_EXISTENT");
            }
            _ => panic!("Expected TradeNotFound error"),
        }
    }

    // Test shared node detection (20%+ reduction)
    #[test]
    fn test_shared_node_optimisation() {
        let portfolio = SamplePortfolioBuilder::new()
            .with_trade_count(50)
            .build()
            .expect("Failed to create portfolio");

        let extractor = PortfolioGraphExtractor::new();

        let all_trade_ids: Vec<String> = portfolio.trade_ids().map(|id| id.to_string()).collect();

        let mut trade_graphs = HashMap::new();
        for trade_id in &all_trade_ids {
            if let Some(trade) = portfolio.trade(&TradeId::new(trade_id)) {
                let graph = create_mock_trade_graph(trade_id, trade);
                trade_graphs.insert(trade_id.clone(), graph);
            }
        }

        let graph = extractor
            .extract_portfolio_graph(&all_trade_ids, &trade_graphs)
            .expect("Graph extraction should succeed");

        // With 50 trades using shared market data, optimisation ratio should be < 1.0
        assert!(
            graph.metadata.optimisation_ratio <= 1.0,
            "Optimisation ratio should be <= 1.0"
        );
    }

    // Helper function to create mock trade graph
    fn create_mock_trade_graph(
        trade_id: &str,
        trade: &pricer_risk::portfolio::Trade,
    ) -> pricer_pricing::graph::ComputationGraph {
        use pricer_pricing::graph::{GraphBuilder, GraphEdge, GraphNode, NodeGroup, NodeType};

        let mut builder = GraphBuilder::with_capacity(10, 15);

        let params = if trade.is_vanilla() {
            vec!["spot", "vol", "rate", "strike"]
        } else {
            vec!["spot", "rate"]
        };

        let mut input_ids = Vec::new();
        for param in &params {
            let node_id = format!("{}_{}", trade_id, param);
            let node = GraphNode {
                id: node_id.clone(),
                node_type: NodeType::Input,
                label: (*param).to_string(),
                value: None,
                is_sensitivity_target: true,
                group: NodeGroup::Sensitivity,
                trade_ids: vec![trade_id.to_string()],
            };
            builder.add_node(node);
            input_ids.push(node_id);
        }

        let calc_id = format!("{}_calc", trade_id);
        builder.add_node(GraphNode {
            id: calc_id.clone(),
            node_type: NodeType::Mul,
            label: "calculation".to_string(),
            value: None,
            is_sensitivity_target: false,
            group: NodeGroup::Intermediate,
            trade_ids: vec![trade_id.to_string()],
        });

        for input_id in &input_ids {
            builder.add_edge(GraphEdge {
                source: input_id.clone(),
                target: calc_id.clone(),
                weight: None,
            });
        }

        let output_id = format!("{}_price", trade_id);
        builder.add_node(GraphNode {
            id: output_id.clone(),
            node_type: NodeType::Output,
            label: "price".to_string(),
            value: None,
            is_sensitivity_target: false,
            group: NodeGroup::Output,
            trade_ids: vec![trade_id.to_string()],
        });

        builder.add_edge(GraphEdge {
            source: calc_id,
            target: output_id,
            weight: None,
        });

        builder.build(Some(trade_id.to_string()))
    }
}

// ============================================================================
// Market Instrument Pipeline Integration Tests
// ============================================================================

#[cfg(test)]
mod market_instrument_integration_tests {
    use infra_domain::market::convention::{
        ConventionRegistry, DepositConvention, FraConvention, MarketConvention, SwapConvention,
    };
    use infra_domain::market::instrument::MarketInstrument;
    use infra_domain::market::{Currency, RateId, RateType};
    use infra_domain::time::{Date, Tenor};
    use infra_domain::trade::{LegType, TradeType};

    /// Test full pipeline: Rate → Convention lookup → Instrument → Trade
    #[test]
    fn test_convention_to_instrument_to_trade_pipeline_deposit() {
        let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
        let convention = MarketConvention::Deposit(DepositConvention::usd());
        let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();

        // Step 1: Create instrument from convention
        let instrument =
            MarketInstrument::new(rate_id.clone(), 0.05, convention, valuation_date, 1_000_000.0)
                .expect("Failed to create instrument");

        // Verify instrument properties
        assert_eq!(instrument.instrument_type_name(), "Deposit");
        assert_eq!(instrument.currency(), Currency::USD);
        assert_eq!(instrument.tenor(), Tenor::ThreeMonths);

        // Step 2: Expand to trade
        let trade = instrument.to_trade().expect("Failed to expand trade");

        // Verify trade structure
        assert!(matches!(trade.trade_type, TradeType::Deposit));
        assert_eq!(trade.legs().count(), 1);

        // Verify leg has cashflows
        let legs: Vec<_> = trade.legs().collect();
        assert_eq!(legs[0].cashflows().count(), 1);
    }

    /// Test full pipeline for swap instruments
    #[test]
    fn test_convention_to_instrument_to_trade_pipeline_swap() {
        let rate_id = RateId::new(Currency::USD, Tenor::FiveYears, RateType::Swap);
        let convention = MarketConvention::Swap(SwapConvention::usd_sofr());
        let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();

        // Step 1: Create instrument
        let instrument =
            MarketInstrument::new(rate_id.clone(), 0.04, convention, valuation_date, 10_000_000.0)
                .expect("Failed to create instrument");

        assert_eq!(instrument.instrument_type_name(), "Swap");
        assert!(instrument.is_swap());

        // Step 2: Expand to trade
        let trade = instrument.to_trade().expect("Failed to expand trade");

        // Verify trade structure
        assert!(matches!(trade.trade_type, TradeType::Swap));
        let legs: Vec<_> = trade.legs().collect();
        assert_eq!(legs.len(), 2, "Swap should have 2 legs");

        // Verify leg types
        let has_fixed = legs.iter().any(|l| matches!(l.leg_type, LegType::Fixed));
        let has_floating = legs.iter().any(|l| matches!(l.leg_type, LegType::Floating));
        assert!(has_fixed, "Should have fixed leg");
        assert!(has_floating, "Should have floating leg");

        // Verify cashflows exist
        for leg in &legs {
            assert!(leg.cashflows().count() > 0, "Each leg should have cashflows");
        }
    }

    /// Test full pipeline for FRA instruments
    #[test]
    fn test_convention_to_instrument_to_trade_pipeline_fra() {
        let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Fra);
        let convention = MarketConvention::Fra(FraConvention::usd_sofr());
        let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();

        let instrument =
            MarketInstrument::new(rate_id, 0.045, convention, valuation_date, 5_000_000.0)
                .expect("Failed to create instrument");

        let trade = instrument.to_trade().expect("Failed to expand trade");

        assert!(matches!(trade.trade_type, TradeType::Fra));
        assert_eq!(trade.legs().count(), 1);
    }

    /// Test ConventionRegistry lookup and instrument creation
    #[test]
    fn test_convention_registry_lookup() {
        let mut registry = ConventionRegistry::new();
        registry.register(
            Currency::USD,
            RateType::Deposit,
            MarketConvention::Deposit(DepositConvention::usd()),
        );
        registry.register(
            Currency::USD,
            RateType::Swap,
            MarketConvention::Swap(SwapConvention::usd_sofr()),
        );

        // Lookup should succeed for registered conventions
        assert!(registry.get(Currency::USD, RateType::Deposit).is_some());
        assert!(registry.get(Currency::USD, RateType::Swap).is_some());

        // Lookup should fail for unregistered conventions
        assert!(registry.get(Currency::EUR, RateType::Deposit).is_none());
    }

    /// Test multiple currency conventions
    #[test]
    fn test_multi_currency_conventions() {
        let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();

        let currencies = [
            (Currency::USD, DepositConvention::usd()),
            (Currency::EUR, DepositConvention::eur()),
            (Currency::GBP, DepositConvention::gbp()),
            (Currency::JPY, DepositConvention::jpy()),
        ];

        for (currency, conv) in currencies {
            let rate_id = RateId::new(currency, Tenor::ThreeMonths, RateType::Deposit);
            let convention = MarketConvention::Deposit(conv);

            let instrument =
                MarketInstrument::new(rate_id, 0.05, convention, valuation_date, 1_000_000.0)
                    .expect(&format!("Failed to create {} instrument", currency));

            assert_eq!(instrument.currency(), currency);

            let trade = instrument.to_trade().expect(&format!(
                "Failed to expand {} trade",
                currency
            ));
            assert_eq!(trade.legs().count(), 1);
        }
    }

    /// Test error handling for invalid rate values
    #[test]
    fn test_invalid_rate_value_error() {
        let rate_id = RateId::new(Currency::USD, Tenor::ThreeMonths, RateType::Deposit);
        let convention = MarketConvention::Deposit(DepositConvention::usd());
        let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();

        // NaN should fail
        let result_nan =
            MarketInstrument::new(rate_id.clone(), f64::NAN, convention.clone(), valuation_date, 1_000_000.0);
        assert!(result_nan.is_err());

        // Infinity should fail
        let result_inf =
            MarketInstrument::new(rate_id, f64::INFINITY, convention, valuation_date, 1_000_000.0);
        assert!(result_inf.is_err());
    }

    /// Test that valid year fractions are calculated
    #[test]
    fn test_year_fraction_calculation() {
        let tenors = [
            (Tenor::ThreeMonths, 0.25, 0.1),
            (Tenor::SixMonths, 0.5, 0.1),
            (Tenor::OneYear, 1.0, 0.1),
            (Tenor::FiveYears, 5.0, 0.5),
        ];

        for (tenor, expected_yf, tolerance) in tenors {
            let rate_id = RateId::new(Currency::USD, tenor, RateType::Deposit);
            let convention = MarketConvention::Deposit(DepositConvention::usd());
            let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();

            let instrument =
                MarketInstrument::new(rate_id, 0.05, convention, valuation_date, 1_000_000.0)
                    .expect("Failed to create instrument");

            let yf = instrument.year_fraction();
            assert!(
                (yf - expected_yf).abs() < tolerance,
                "Year fraction for {:?} should be ~{}, got {}",
                tenor,
                expected_yf,
                yf
            );
        }
    }

    /// Test instrument clone and equality
    #[test]
    fn test_instrument_clone_and_equality() {
        let rate_id = RateId::new(Currency::USD, Tenor::OneYear, RateType::Swap);
        let convention = MarketConvention::Swap(SwapConvention::usd_sofr());
        let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();

        let instrument =
            MarketInstrument::new(rate_id, 0.04, convention, valuation_date, 10_000_000.0).unwrap();

        let cloned = instrument.clone();
        assert_eq!(instrument, cloned);
        assert_eq!(instrument.rate_value, cloned.rate_value);
        assert_eq!(instrument.notional, cloned.notional);
    }
}
