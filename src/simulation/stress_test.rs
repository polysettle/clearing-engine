//! Stress testing utilities for the clearing engine.
//!
//! Generates random obligation networks to test netting performance
//! under various conditions.
//!
//! # Status: Phase 2 — basic random generation implemented

use crate::core::currency::CurrencyCode;
use crate::core::obligation::{Obligation, ObligationSet};
use crate::core::party::PartyId;
use rand::Rng;
use rust_decimal::Decimal;

/// Configuration for generating a random obligation network.
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Number of parties in the network.
    pub party_count: usize,
    /// Currencies to use.
    pub currencies: Vec<CurrencyCode>,
    /// Average number of obligations per party.
    pub avg_obligations_per_party: usize,
    /// Minimum obligation amount.
    pub min_amount: Decimal,
    /// Maximum obligation amount.
    pub max_amount: Decimal,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            party_count: 10,
            currencies: vec![CurrencyCode::new("USD")],
            avg_obligations_per_party: 3,
            min_amount: Decimal::from(1_000),
            max_amount: Decimal::from(10_000_000),
        }
    }
}

/// Generate a random obligation network for testing.
pub fn generate_random_network(config: &NetworkConfig) -> ObligationSet {
    let mut rng = rand::thread_rng();
    let mut set = ObligationSet::new();

    let parties: Vec<PartyId> = (0..config.party_count)
        .map(|i| PartyId::new(format!("PARTY-{:03}", i)))
        .collect();

    let total_obligations = config.party_count * config.avg_obligations_per_party;

    for _ in 0..total_obligations {
        let debtor_idx = rng.gen_range(0..parties.len());
        let mut creditor_idx = rng.gen_range(0..parties.len());
        while creditor_idx == debtor_idx {
            creditor_idx = rng.gen_range(0..parties.len());
        }

        let currency_idx = rng.gen_range(0..config.currencies.len());

        // Generate random amount between min and max
        let min_f64: f64 = config.min_amount.to_string().parse().unwrap_or(1000.0);
        let max_f64: f64 = config.max_amount.to_string().parse().unwrap_or(10_000_000.0);
        let amount_f64 = rng.gen_range(min_f64..max_f64);
        let amount = Decimal::from_f64_retain(amount_f64)
            .unwrap_or(Decimal::from(1000))
            .round_dp(2);

        if amount > Decimal::ZERO {
            set.add(Obligation::new(
                parties[debtor_idx].clone(),
                parties[creditor_idx].clone(),
                amount,
                config.currencies[currency_idx].clone(),
            ));
        }
    }

    set
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimization::netting::NettingEngine;

    #[test]
    fn test_random_network_generation() {
        let config = NetworkConfig {
            party_count: 5,
            currencies: vec![CurrencyCode::new("USD"), CurrencyCode::new("BRL")],
            avg_obligations_per_party: 3,
            ..Default::default()
        };

        let set = generate_random_network(&config);
        assert!(!set.is_empty());
        assert!(set.len() <= config.party_count * config.avg_obligations_per_party);
    }

    #[test]
    fn test_random_network_netting() {
        let config = NetworkConfig {
            party_count: 20,
            avg_obligations_per_party: 5,
            ..Default::default()
        };

        let set = generate_random_network(&config);
        let result = NettingEngine::multilateral_net(&set);

        assert!(result.is_valid());
        // In a random network, netting should generally save something
        assert!(result.net_total() <= result.gross_total());
    }
}
