//! FX volatility modeling for settlement risk analysis.
//!
//! Models the impact of exchange rate movements on net settlement
//! positions and liquidity requirements.
//!
//! # Status: Phase 2 — interface defined, implementation in progress

use crate::core::currency::CurrencyCode;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of an FX shock scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxShockResult {
    /// Description of the shock applied.
    pub scenario: String,
    /// Net settlement before the shock.
    pub baseline_net: Decimal,
    /// Net settlement after the shock.
    pub shocked_net: Decimal,
    /// Change in net settlement.
    pub impact: Decimal,
}

/// Configuration for FX volatility scenarios.
///
/// Defines shock magnitudes to apply to exchange rates
/// for stress testing settlement positions.
#[derive(Debug, Clone)]
pub struct FxShockConfig {
    /// Shocks to apply: currency pair -> percentage change (e.g., 0.10 = 10% depreciation).
    pub shocks: HashMap<(CurrencyCode, CurrencyCode), Decimal>,
}

// TODO: Phase 2 implementation
// - Apply FX shocks to obligation sets
// - Recompute netting under stressed rates
// - Monte Carlo simulation over rate distributions
// - VaR-style exposure reporting
