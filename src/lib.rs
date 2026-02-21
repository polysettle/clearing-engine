//! # clearing-engine
//!
//! Open multi-currency clearing and liquidity optimization engine.
//!
//! Given a directed graph of payment obligations in multiple currencies,
//! this engine computes optimal netting to minimize required liquidity.
//!
//! ## Architecture
//!
//! - **core** — Foundational types: obligations, currencies, parties, ledger
//! - **graph** — Payment graph, cycle detection, strongly connected components
//! - **optimization** — Bilateral and multilateral netting algorithms
//! - **simulation** — Stress testing and FX volatility modeling

pub mod core;
pub mod graph;
pub mod optimization;
pub mod simulation;

/// Convenience re-exports for common usage.
pub mod prelude {
    pub use crate::core::currency::CurrencyCode;
    pub use crate::core::ledger::Ledger;
    pub use crate::core::obligation::Obligation;
    pub use crate::core::party::PartyId;
    pub use crate::graph::payment_graph::PaymentGraph;
    pub use crate::optimization::netting::{BilateralNettingResult, NettingEngine, NettingResult};
}
