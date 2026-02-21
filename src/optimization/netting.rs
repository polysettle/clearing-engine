use crate::core::currency::CurrencyCode;
use crate::core::ledger::Ledger;
use crate::core::obligation::ObligationSet;
use crate::core::party::PartyId;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of a bilateral netting computation between two parties.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BilateralNettingResult {
    pub party_a: PartyId,
    pub party_b: PartyId,
    pub currency: CurrencyCode,
    /// Gross amount A owes B.
    pub gross_a_to_b: Decimal,
    /// Gross amount B owes A.
    pub gross_b_to_a: Decimal,
    /// Net amount: positive means A owes B net, negative means B owes A net.
    pub net_amount: Decimal,
    /// Liquidity saved by netting.
    pub savings: Decimal,
}

/// Result of a multilateral netting computation across all parties.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NettingResult {
    /// Net position of each party in each currency.
    ledger: Ledger,
    /// Gross total before netting.
    gross_total: Decimal,
    /// Net total after netting.
    net_total: Decimal,
    /// Per-currency breakdown.
    currency_breakdown: HashMap<CurrencyCode, CurrencyNettingResult>,
}

impl NettingResult {
    /// Total gross obligations before netting.
    pub fn gross_total(&self) -> Decimal {
        self.gross_total
    }

    /// Total net settlement required after netting.
    pub fn net_total(&self) -> Decimal {
        self.net_total
    }

    /// Absolute liquidity saved.
    pub fn savings(&self) -> Decimal {
        self.gross_total - self.net_total
    }

    /// Savings as a percentage of gross.
    pub fn savings_percent(&self) -> f64 {
        if self.gross_total == Decimal::ZERO {
            return 0.0;
        }
        let savings = self.gross_total - self.net_total;
        // Convert to f64 for percentage display
        let pct = savings * Decimal::from(100) / self.gross_total;
        pct.to_string().parse::<f64>().unwrap_or(0.0)
    }

    /// The resulting ledger with net positions.
    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    /// Get the net position of a specific party in a specific currency.
    pub fn net_position(&self, party: &PartyId, currency: &CurrencyCode) -> Decimal {
        self.ledger.position(party, currency)
    }

    /// Per-currency breakdown of netting results.
    pub fn currency_breakdown(&self) -> &HashMap<CurrencyCode, CurrencyNettingResult> {
        &self.currency_breakdown
    }

    /// Verify the result is valid (ledger is balanced).
    pub fn is_valid(&self) -> bool {
        self.ledger.is_balanced()
    }
}

/// Netting result for a single currency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyNettingResult {
    pub currency: CurrencyCode,
    pub gross_total: Decimal,
    pub net_total: Decimal,
    pub party_count: usize,
}

impl CurrencyNettingResult {
    pub fn savings(&self) -> Decimal {
        self.gross_total - self.net_total
    }

    pub fn savings_percent(&self) -> f64 {
        if self.gross_total == Decimal::ZERO {
            return 0.0;
        }
        let pct = self.savings() * Decimal::from(100) / self.gross_total;
        pct.to_string().parse::<f64>().unwrap_or(0.0)
    }
}

/// The core netting engine.
///
/// Provides algorithms for bilateral and multilateral netting
/// of payment obligations.
pub struct NettingEngine;

impl NettingEngine {
    /// Perform bilateral netting between two specific parties in one currency.
    ///
    /// Bilateral netting offsets mutual obligations between a pair.
    /// If A owes B $100 and B owes A $60, the net obligation is A owes B $40.
    pub fn bilateral_net(
        obligations: &ObligationSet,
        party_a: &PartyId,
        party_b: &PartyId,
        currency: &CurrencyCode,
    ) -> BilateralNettingResult {
        let mut a_to_b = Decimal::ZERO;
        let mut b_to_a = Decimal::ZERO;

        for ob in obligations.obligations() {
            if ob.currency() != currency {
                continue;
            }
            if ob.debtor() == party_a && ob.creditor() == party_b {
                a_to_b += ob.amount();
            } else if ob.debtor() == party_b && ob.creditor() == party_a {
                b_to_a += ob.amount();
            }
        }

        let net = a_to_b - b_to_a;
        let gross = a_to_b + b_to_a;
        let net_settlement = net.abs();
        let savings = gross - net_settlement;

        BilateralNettingResult {
            party_a: party_a.clone(),
            party_b: party_b.clone(),
            currency: currency.clone(),
            gross_a_to_b: a_to_b,
            gross_b_to_a: b_to_a,
            net_amount: net,
            savings,
        }
    }

    /// Perform multilateral netting across all parties and currencies.
    ///
    /// Multilateral netting computes each party's net position against
    /// the entire system. This achieves maximal netting efficiency
    /// for a given set of obligations.
    ///
    /// # Algorithm
    ///
    /// 1. Build a ledger by applying all obligations.
    /// 2. Each party's net position = sum(incoming) - sum(outgoing).
    /// 3. Net settlement = sum of all positive positions (= sum of |negative|).
    /// 4. Savings = gross - net.
    ///
    /// The ledger is guaranteed to be balanced: sum of all positions = 0.
    pub fn multilateral_net(obligations: &ObligationSet) -> NettingResult {
        let mut ledger = Ledger::new();
        let mut gross_total = Decimal::ZERO;

        // Per-currency tracking
        let mut currency_gross: HashMap<CurrencyCode, Decimal> = HashMap::new();
        let mut currency_parties: HashMap<CurrencyCode, HashMap<PartyId, bool>> = HashMap::new();

        for ob in obligations.obligations() {
            ledger.apply_obligation(ob);
            gross_total += ob.amount();

            *currency_gross
                .entry(ob.currency().clone())
                .or_insert(Decimal::ZERO) += ob.amount();

            let parties = currency_parties
                .entry(ob.currency().clone())
                .or_default();
            parties.insert(ob.debtor().clone(), true);
            parties.insert(ob.creditor().clone(), true);
        }

        let net_total = ledger.total_net_settlement();

        // Build per-currency breakdown
        let mut currency_breakdown = HashMap::new();
        for (currency, gross) in &currency_gross {
            // Compute net for this currency specifically
            let mut currency_net = Decimal::ZERO;
            for ((_, cur), amount) in ledger.all_positions() {
                if cur == currency && *amount > Decimal::ZERO {
                    currency_net += amount;
                }
            }

            let party_count = currency_parties
                .get(currency)
                .map(|p| p.len())
                .unwrap_or(0);

            currency_breakdown.insert(
                currency.clone(),
                CurrencyNettingResult {
                    currency: currency.clone(),
                    gross_total: *gross,
                    net_total: currency_net,
                    party_count,
                },
            );
        }

        NettingResult {
            ledger,
            gross_total,
            net_total,
            currency_breakdown,
        }
    }
}

impl std::fmt::Display for NettingResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Netting Result ===")?;
        writeln!(f, "Gross Total:    {}", self.gross_total)?;
        writeln!(f, "Net Total:      {}", self.net_total)?;
        writeln!(f, "Savings:        {}", self.savings())?;
        writeln!(f, "Savings %:      {:.1}%", self.savings_percent())?;
        writeln!(f, "Valid:          {}", self.is_valid())?;

        for (currency, breakdown) in &self.currency_breakdown {
            writeln!(f, "\n--- {} ---", currency)?;
            writeln!(f, "  Gross:   {}", breakdown.gross_total)?;
            writeln!(f, "  Net:     {}", breakdown.net_total)?;
            writeln!(f, "  Parties: {}", breakdown.party_count)?;
            writeln!(f, "  Savings: {:.1}%", breakdown.savings_percent())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::obligation::Obligation;
    use rust_decimal_macros::dec;

    #[test]
    fn test_bilateral_netting() {
        let mut set = ObligationSet::new();
        let usd = CurrencyCode::new("USD");
        let a = PartyId::new("A");
        let b = PartyId::new("B");

        set.add(Obligation::new(a.clone(), b.clone(), dec!(100), usd.clone()));
        set.add(Obligation::new(b.clone(), a.clone(), dec!(60), usd.clone()));

        let result = NettingEngine::bilateral_net(&set, &a, &b, &usd);
        assert_eq!(result.gross_a_to_b, dec!(100));
        assert_eq!(result.gross_b_to_a, dec!(60));
        assert_eq!(result.net_amount, dec!(40)); // A owes B net $40
        assert_eq!(result.savings, dec!(120)); // Gross 160, net 40, saved 120
    }

    #[test]
    fn test_perfect_cycle_netting() {
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
            dec!(100),
            usd.clone(),
        ));
        set.add(Obligation::new(
            PartyId::new("C"),
            PartyId::new("A"),
            dec!(100),
            usd.clone(),
        ));

        let result = NettingEngine::multilateral_net(&set);
        assert_eq!(result.gross_total(), dec!(300));
        assert_eq!(result.net_total(), Decimal::ZERO);
        assert_eq!(result.savings(), dec!(300));
        assert!((result.savings_percent() - 100.0).abs() < 0.01);
        assert!(result.is_valid());
    }

    #[test]
    fn test_partial_netting() {
        let mut set = ObligationSet::new();
        let usd = CurrencyCode::new("USD");

        // A owes B 100, B owes C 60, C owes A 30
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
        set.add(Obligation::new(
            PartyId::new("C"),
            PartyId::new("A"),
            dec!(30),
            usd.clone(),
        ));

        let result = NettingEngine::multilateral_net(&set);
        assert_eq!(result.gross_total(), dec!(190));
        // A: -100 + 30 = -70 (owes 70)
        // B: +100 - 60 = +40 (owed 40)
        // C: +60 - 30 = +30 (owed 30)
        // Net = 40 + 30 = 70
        assert_eq!(result.net_total(), dec!(70));
        assert!(result.is_valid());
    }

    #[test]
    fn test_multi_currency_netting() {
        let mut set = ObligationSet::new();
        let usd = CurrencyCode::new("USD");
        let brl = CurrencyCode::new("BRL");

        // USD cycle
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

        // BRL: no cycle
        set.add(Obligation::new(
            PartyId::new("A"),
            PartyId::new("B"),
            dec!(500),
            brl.clone(),
        ));

        let result = NettingEngine::multilateral_net(&set);
        assert_eq!(result.gross_total(), dec!(700));
        // USD nets to 0, BRL nets to 500
        assert_eq!(result.net_total(), dec!(500));
        assert!(result.is_valid());

        let usd_breakdown = &result.currency_breakdown()[&usd];
        assert_eq!(usd_breakdown.net_total, Decimal::ZERO);

        let brl_breakdown = &result.currency_breakdown()[&brl];
        assert_eq!(brl_breakdown.net_total, dec!(500));
    }

    #[test]
    fn test_empty_obligations() {
        let set = ObligationSet::new();
        let result = NettingEngine::multilateral_net(&set);
        assert_eq!(result.gross_total(), Decimal::ZERO);
        assert_eq!(result.net_total(), Decimal::ZERO);
        assert!(result.is_valid());
    }

    #[test]
    fn test_large_network() {
        let mut set = ObligationSet::new();
        let usd = CurrencyCode::new("USD");

        // Create a 5-party network with various obligations
        let parties = ["A", "B", "C", "D", "E"];
        for i in 0..parties.len() {
            for j in 0..parties.len() {
                if i != j {
                    set.add(Obligation::new(
                        PartyId::new(parties[i]),
                        PartyId::new(parties[j]),
                        Decimal::from((i + 1) * (j + 1) * 10),
                        usd.clone(),
                    ));
                }
            }
        }

        let result = NettingEngine::multilateral_net(&set);
        assert!(result.is_valid());
        // Net should be significantly less than gross
        assert!(result.net_total() < result.gross_total());
        assert!(result.savings_percent() > 0.0);
    }
}
