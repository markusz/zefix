//! The MCP surface: tool names, descriptions and parameter schemas for the assistant.

use crate::tools;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ServerCapabilities, ServerConfig};
use rmcp::schemars::{self, JsonSchema};
use rmcp::{ServerHandler, tool, tool_handler, tool_router};
use serde::Deserialize;

const INSTRUCTIONS: &str = "\
zefix! computes Schafkopf probabilities (long deck, 32 cards) by Monte Carlo simulation.
1. Call `reference` once: it documents the TOML scenario format and the query language \
(German keywords such as trumpf, ober, sau, frei).
2. Write a scenario: the game, what the user knows about their own hand (fixed cards and/or \
slots such as 2 of unter), and optionally conditions on the deal as `[me] where`, which also \
covers other players (e.g. \"left.frei(gras)\").
3. Add one [[query]] per question, then call `simulate`.
Each query's `p` is the share of the simulated deals that match the scenario on which the \
query holds. Use `check` to validate a scenario without simulating; errors name the offending \
spot, so fix and retry.";

#[derive(Deserialize, JsonSchema)]
struct SimulateParams {
    /// Scenario and queries as TOML, in the format `reference` describes.
    scenario: String,
    /// Deals to simulate; default 1,000,000, at most 10,000,000. Overrides `iterations` in the TOML.
    iterations: Option<u64>,
}

#[derive(Deserialize, JsonSchema)]
struct CheckParams {
    /// Scenario and queries as TOML, in the format `reference` describes.
    scenario: String,
}

/// Stateless: every call carries its whole scenario.
#[derive(Clone)]
pub struct Zefix;

#[tool_router]
impl Zefix {
    #[tool(
        description = "Documentation of the TOML scenario format and the query language (German), with worked examples. Read it before writing a scenario."
    )]
    async fn reference(&self) -> String {
        tools::reference()
    }

    #[tool(
        description = "Validate a scenario without simulating; returns the game, the hand and the number of queries, or the error."
    )]
    async fn check(&self, Parameters(params): Parameters<CheckParams>) -> Result<String, String> {
        tools::check(&params.scenario)
    }

    #[tool(
        description = "Simulate a scenario. Returns JSON: per query `p` (0..1), its 95 % Wilson interval `ci95` and `hits`, plus how many deals matched the scenario."
    )]
    async fn simulate(
        &self,
        Parameters(params): Parameters<SimulateParams>,
    ) -> Result<String, String> {
        // Simulating is CPU-bound; keep it off the protocol's async workers.
        let report = tokio::task::spawn_blocking(move || {
            tools::simulate(&params.scenario, params.iterations)
        })
        .await
        .map_err(|e| e.to_string())??;
        serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
    }
}

#[tool_handler]
impl ServerHandler for Zefix {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("zefix", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }
}
