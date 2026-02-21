use crate::core::currency::CurrencyCode;
use crate::core::party::PartyId;
use crate::optimization::netting::NettingResult;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Analysis of liquidity requirements for settlement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityAnalysis {
    /// Minimum liquidity each debtor needs to fund their net position.
    pub debtor_requirements: HashMap<PartyId, HashMap<CurrencyCode, Decimal>>,
    /// Total liquidity required across all debtors per currency.
    pub total_required: HashMap<CurrencyCode, Decimal>,
    /// Liquidity saved compared to gross settlement.
    pub gross_requirement: Decimal,
    /// Net liquidity requirement.
    pub net_requirement: Decimal,
}

impl LiquidityAnalysis {
    /// Compute liquidity requirements from a netting result.
    pub fn from_netting_result(result: &NettingResult) -> Self {
        let mut debtor_requirements: HashMap<PartyId, HashMap<CurrencyCode, Decimal>> =
            HashMap::new();
        let mut total_required: HashMap<CurrencyCode, Decimal> = HashMap::new();

        for ((party, currency), amount) in result.ledger().all_positions() {
            if *amount < Decimal::ZERO {
                // This party is a net debtor — they need liquidity
                let abs_amount = amount.abs();
                debtor_requirements
                    .entry(party.clone())
                    .or_default()
                    .insert(currency.clone(), abs_amount);
                *total_required
                    .entry(currency.clone())
                    .or_insert(Decimal::ZERO) += abs_amount;
            }
        }

        LiquidityAnalysis {
            debtor_requirements,
            total_required,
            gross_requirement: result.gross_total(),
            net_requirement: result.net_total(),
        }
    }

    /// Liquidity savings ratio.
    pub fn savings_ratio(&self) -> f64 {
        if self.gross_requirement == Decimal::ZERO {
            return 0.0;
        }
        let ratio = (self.gross_requirement - self.net_requirement) / self.gross_requirement;
        ratio.to_string().parse::<f64>().unwrap_or(0.0)
    }
}

impl std::fmt::Display for LiquidityAnalysis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Liquidity Analysis ===")?;
        writeln!(f, "Gross Requirement: {}", self.gross_requirement)?;
        writeln!(f, "Net Requirement:   {}", self.net_requirement)?;
        writeln!(f, "Savings Ratio:     {:.1}%", self.savings_ratio() * 100.0)?;

        writeln!(f, "\nPer-Currency Requirements:")?;
        for (currency, amount) in &self.total_required {
            writeln!(f, "  {}: {}", currency, amount)?;
        }

        writeln!(f, "\nDebtor Requirements:")?;
        for (party, currencies) in &self.debtor_requirements {
            for (currency, amount) in currencies {
                writeln!(f, "  {} needs {} {}", party, amount, currency)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::obligation::{Obligation, ObligationSet};
    use crate::optimization::netting::NettingEngine;
    use rust_decimal_macros::dec;

    #[test]
    fn test_liquidity_analysis_basic() {
        let mut set = ObligationSet::new();
        let usd = CurrencyCode::new("USD");

        set.add(Obligation::new(
            PartyId::new("A"),
            PartyId::new("B"),
            dec!(100),
            usd.clone(),
        ));
        set.add(Obligation::new(
            PartyId::new("B"),
            PartyId::new("C"),
            dec!(60),
            usd.clone(),
        ));

        let netting = NettingEngine::multilateral_net(&set);
        let analysis = LiquidityAnalysis::from_netting_result(&netting);

        assert_eq!(analysis.gross_requirement, dec!(160));
        assert!(analysis.savings_ratio() >= 0.0);
    }

    #[test]
    fn test_liquidity_perfect_cycle() {
        let mut set = ObligationSet::new();
        let usd = CurrencyCode::new("USD");

        set.add(Obligation::new(
            PartyId::new("A"),
            PartyId::new("B"),
            dec!(100),
            usd.clone(),
        ));
        set.add(Obligation::new(
            PartyId::new("B"),
            PartyId::new("A"),
            dec!(100),
            usd.clone(),
        ));

        let netting = NettingEngine::multilateral_net(&set);
        let analysis = LiquidityAnalysis::from_netting_result(&netting);

        assert_eq!(analysis.net_requirement, Decimal::ZERO);
        assert!((analysis.savings_ratio() - 1.0).abs() < 0.001);
    }
}
