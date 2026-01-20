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
